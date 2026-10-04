/*
 * MIT License
 *
 * Copyright (c) 2023 Frédéric Bour <frederic.bour@lakaban.net>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to
 * deal in the Software without restriction, including without limitation the
 * rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
 * sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
 * IN THE SOFTWARE.
 */

/* Pitex modifications (2026), derived from TeXpresso src/frontend/engine_tex.c
 * at e8df7709077b2f86f6e16e6c86ceefb86de06f8d:
 * - MuPDF fz_buffer/fz_context/fz_try replaced by the Pitex tbuf type;
 * - the incremental MuPDF XDV renderer (incdvi) replaced by the Pitex XDV
 *   page index; the frontend incremental SyncTeX parser and editor/SDL
 *   notifications removed (the driver reads the VFS buffers at publish time);
 * - the generic txp_engine class removed, functions exported directly;
 * - engine stderr redirected by the caller, root process reaped;
 * - watchdog support (idle time, kill running process);
 * - macOS font barrier: checkpoint children that must load a new platform
 *   font report Q_FNTB; checkpoints are then delayed past that point.
 * The fork-checkpoint, trace/fence, rollback and convergence logic is
 * TeXpresso's. */

#include <stdlib.h>
#include <limits.h>
#include <unistd.h>
#include <string.h>
#include <fcntl.h>
#include <errno.h>
#include <time.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <sys/socket.h>
#include <signal.h>
#ifdef __linux__
#include <sys/prctl.h>
#endif
#include "engine_tex.h"
#include "xdv.h"
#include "state.h"

typedef struct
{
  fileentry_t *entry;
  int position;
} fence_t;

typedef struct
{
  fileentry_t *entry;
  int seen, time;
} trace_entry_t;

typedef struct
{
  int pid, fd;
  int trace_len;
  mark_t snap;
  bool forked;
} process_t;

enum
{
  MAX_PROCESS = 32
};

struct tex_engine
{
  char *name;
  char *engine_path;
  char *inclusion_path;

  filesystem_t *fs;
  state_t st;
  log_t *log;

  channel_t *c;
  process_t processes[MAX_PROCESS];
  int process_count;

  trace_entry_t *trace;
  int trace_cap;
  fence_t fences[16];
  int fence_pos;
  mark_t restart;

  xdv_index *dvi;
  fileentry_t *log_entry;

  struct {
    int trace_len, offset, flush;
  } rollback;

  bool aux_dirty;
  bool finishing;

  /* Pitex */
  int root_pid;
  int last_status;
  struct timespec last_query;
  int snapshot_barrier;
  bool barrier_pending;
};

static int mini(int a, int b) { return a < b ? a : b; }
static int maxi(int a, int b) { return a > b ? a : b; }

static void touch_query_clock(struct tex_engine *self)
{
  clock_gettime(CLOCK_MONOTONIC, &self->last_query);
}

// Backtrackable process state & VFS representation

static process_t *get_process(struct tex_engine *t)
{
  if (t->process_count == 0)
    mabort();
  return &t->processes[t->process_count-1];
}

// Useful routines

static char *last_index(char *path, char needle)
{
  char *result = path;
  while (*path)
  {
    if (*path == needle)
      result = path + 1;
    path += 1;
  }
  return result;
}

static void answer_query(struct tex_engine *self, query_t *q);

// Launching processes

static pid_t exec_xelatex_generic(char **args, int *fd)
{
  int sockets[2];
  if (socketpair(PF_UNIX, SOCK_STREAM, 0, sockets) != 0)
  {
    perror("exec_xelatex socketpair");
    mabort();
  }

  char buf[30];
  snprintf(buf, 30, "%d", sockets[1]);
  setenv("TEXPRESSO_FD", buf, 1);

#ifdef __APPLE__
  static int env_init = 0;
  if (!env_init)
  {
    env_init = 1;
    setenv("OBJC_DISABLE_INITIALIZE_FORK_SAFETY", "YES", 1);
  }
#endif

  pid_t pid = fork();

  if (pid == -1)
  {
    perror("exec_xelatex fork");
    mabort();
  }

  if (pid == 0)
  {
    /* CHILD */
    close(sockets[0]);
#ifdef __linux__
    // Pitex: the exec'd root engine never outlives the driver.
    prctl(PR_SET_PDEATHSIG, SIGKILL);
#endif
    // Redirect stdout to stderr
    dup2(STDERR_FILENO, STDOUT_FILENO);
    execv(args[0], args);
    _exit(2);
  }

  /* PARENT */
  if (close(sockets[1]) != 0)
    mabort();
  fcntl(sockets[0], F_SETFD, FD_CLOEXEC);
  *fd = sockets[0];
  return pid;
}

static pid_t exec_xelatex(char *engine_path, const char *filename, int *fd)
{
  char *args[] = {
    engine_path,
    "-texpresso",
    (char*)filename,
    NULL
  };

  pid_t pid = exec_xelatex_generic(args, fd);
  fprintf(stderr, "[process] launched pid %d (using %s)\n", pid, engine_path);
  return pid;
}

static void reap_root(struct tex_engine *self, bool wait)
{
  if (self->root_pid <= 0)
    return;
  int status;
  pid_t r;
  do
    r = waitpid(self->root_pid, &status, wait ? 0 : WNOHANG);
  while (r == -1 && errno == EINTR);
  if (r == self->root_pid)
  {
    self->last_status = status;
    self->root_pid = 0;
  }
}

static void prepare_process(struct tex_engine *self)
{
  if (self->process_count == 0)
  {
    reap_root(self, false);
    log_rollback(self->log, self->restart);
    self->process_count = 1;
    process_t *p = get_process(self);
    p->pid = exec_xelatex(self->engine_path, self->name, &p->fd);
    p->trace_len = 0;
    p->forked = false;
    self->root_pid = p->pid;
    touch_query_clock(self);
    if (!channel_handshake(self->c, p->fd))
      mabort();
  }
}

// Terminating processes

static void close_process(process_t *p)
{
  if (p->fd != -1)
  {
    kill(p->pid, SIGTERM);
    close(p->fd);
    p->fd = -1;
  }
}

static void pop_process(struct tex_engine *self)
{
  process_t *p = get_process(self);
  close_process(p);
  channel_reset(self->c);
  self->process_count -= 1;
  mark_t mark =
    self->process_count > 0 ? get_process(self)->snap : self->restart;
  log_rollback(self->log, mark);
  if (self->process_count == 0)
    reap_root(self, true);
}

static void clear_convergence_stash(struct tex_engine *self)
{
  fileentry_t *e;
  for (int index = 0; (e = filesystem_scan(self->fs, &index));)
  {
    if (e->edit_data_from_convergence && e->edit_data)
    {
      tbuf_drop(e->edit_data);
      e->edit_data = NULL;
      e->edit_data_from_convergence = false;
    }
  }
  self->aux_dirty = false;
  self->finishing = false;
}

static bool read_query(struct tex_engine *self, channel_t *t, query_t *q)
{
  process_t *p = get_process(self);
  bool result = channel_read_query(t, p->fd, q);
  if (!result)
  {
    fprintf(stderr, "[process] terminating process\n");
    close_process(p);
  }
  return result;
}

static void decimate_processes(struct tex_engine *self)
{
  bool keep[MAX_PROCESS] = {0,};

  int target = 32;
  for  (int i = 0; i < self->process_count; ++i)
  {
    process_t *p = &self->processes[i];
    if (p->trace_len >= target)
    {
        keep[i] = true;
        target *= 2;
    }
  }

  target = self->processes[self->process_count - 1].trace_len;
  int delta = 32;
  for (int i = self->process_count - 1; i >= 0; --i)
  {
    process_t *p = &self->processes[i];
    if (p->trace_len <= target)
    {
      keep[i] = true;
      delta *= 2;
      target -= delta;
    }
    else if (keep[i])
    {
      delta *= 2;
      target = p->trace_len - delta;
    }
  }

  int i = 0;
  for (int j = 0; j < self->process_count; j++)
  {
    if (!keep[j])
    {
      close_process(&self->processes[j]);
      continue;
    }
    if (i != j)
      self->processes[i] = self->processes[j];
    i++;
  }
  self->process_count = i;
  fprintf(stderr, "[process] decimated snapshots to %d\n", self->process_count);
}

// Engine implementation

void txp_engine_free(tex_engine *self)
{
  while (self->process_count > 0)
    pop_process(self);
  reap_root(self, true);
  xdv_index_free(self->dvi);
  filesystem_free(self->fs);
  log_free(self->log);
  channel_free(self->c);
  free(self->trace);
  free(self->name);
  free(self->engine_path);
  free(self->inclusion_path);
  free(self);
}

static const char *expand_path(const char **inclusion_path, const char *name, char buffer[1024])
{
  if (!*inclusion_path || !(*inclusion_path)[0])
    return NULL;

  if (name[0] == '/')
    return NULL;

  if (name[0] == '.' && name[1] == '/')
  {
    name += 2;
    while (*name == '/')
      name += 1;
  }

  char *p = buffer;
  const char *i = *inclusion_path;

  while (*i)
  {
    if (p >= buffer + 1023) return NULL;
    *p = *i;
    p += 1;
    i += 1;
  }
  *inclusion_path = i+1;

  if (p > buffer && p[-1] != '/')
  {
    if (p >= buffer + 1023) return NULL;
    p[0] = '/';
    p += 1;
  }

  while (*name)
  {
    if (p >= buffer + 1023) return NULL;
    *p = *name;
    p += 1;
    name += 1;
  }

  *p = '\0';

  return buffer;
}

static void check_fid(file_id fid)
{
  if (fid < 0 || fid >= MAX_FILES)
    mabort();
}

static void record_seen(struct tex_engine *self, fileentry_t *entry, int seen, int time)
{
  process_t *p = get_process(self);

  if (p->trace_len > 0 && self->trace[p->trace_len-1].entry == entry &&
      (self->process_count <= 1 ||
      self->processes[self->process_count - 2].trace_len != p->trace_len))
  {
    self->trace[p->trace_len-1].time = time;
    entry->seen = seen;
    return;
  }

  if (p->trace_len == self->trace_cap)
  {
    int new_cap = self->trace_cap == 0 ? 8 : self->trace_cap * 2;
    trace_entry_t *newtr = calloc(sizeof(trace_entry_t), new_cap);
    if (newtr == NULL) abort();
    if (self->trace)
    {
      memcpy(newtr, self->trace, self->trace_cap * sizeof(trace_entry_t));
      free(self->trace);
    }
    self->trace = newtr;
    self->trace_cap = new_cap;
  }

  self->trace[p->trace_len] = (trace_entry_t){
    .entry = entry,
    .seen = entry->seen,
    .time = time,
  };
  entry->seen = seen;
  p->trace_len += 1;
}

static tbuf *entry_data(fileentry_t *e)
{
  if (e->saved.data)
    return e->saved.data;
  if (e->edit_data)
    return e->edit_data;
  return e->fs_data;
}

static const char *
lookup_path(struct tex_engine *self, const char *path, char buf[1024], struct stat *st)
{
  struct stat st1;
  if (st == NULL)
    st = &st1;

  const char *fs_path = path;
  const char *inclusion_path = self->inclusion_path;

  do {
    if (stat(fs_path, st) != -1 && S_ISREG(st->st_mode))
      break;
  }
  while ((fs_path = expand_path(&inclusion_path, path, buf)));

  return fs_path;
}

static bool need_snapshot(struct tex_engine *self, int time)
{
  // Fences are pending: don't snapshot now
  if (self->fence_pos != -1)
    return 0;

  int process = self->process_count - 1;

  // Pitex: no checkpoint before a recorded platform-font barrier (macOS).
  if (self->processes[process].trace_len <= self->snapshot_barrier)
    return 0;

  int last_time;

  if (process > 0)
  {
    // There is already some snapshot, stop if no new event has been traced
    if (self->processes[process].trace_len == self->processes[process-1].trace_len)
      return 0;

    last_time = self->trace[self->processes[process-1].trace_len - 1].time;
  }
  else
  {
    #ifdef __APPLE__
    // Workaround for macOS (TeXpresso): system fonts cannot be loaded after
    // fork without exec, so delay the first fork until output started,
    // hoping that all fonts have been loaded at that point. Fonts loaded
    // later are handled by the Pitex font barrier.
    if (!xdv_index_output_started(self->dvi))
      return 0;
    #endif

    // No snapshot, measure time since root process started
    last_time = 0;
  }

  return time > 500 + last_time;
}

static void answer_query(struct tex_engine *self, query_t *q)
{
  process_t *p = get_process(self);
  answer_t a;
  switch (q->tag)
  {
    case Q_OPRD:
    case Q_OPWR:
    {
      check_fid(q->open.fid);
      filecell_t *cell = &self->st.table[q->open.fid];
      if (cell->entry != NULL) mabort();

      fileentry_t *e = NULL;

      char fs_path_buffer[1024];
      const char *fs_path = NULL;

      if (q->tag == Q_OPRD)
      {
        e = filesystem_lookup(self->fs, q->open.path);
        if (!e || !entry_data(e))
        {
          fs_path = lookup_path(self, q->open.path, fs_path_buffer, NULL);
          if (!fs_path && !(e && e->edit_data))
          {
            // File is missing: record this observation and mark the lookup
            // as failed.
            e = filesystem_lookup_or_create(self->fs, q->open.path);
            log_fileentry(self->log, e);
            record_seen(self, e, INT_MAX, q->time);
            a.tag = A_PASS;
            channel_write_answer(self->c, p->fd, &a);
            break;
          }
        }
      }

      if (!e)
        e = filesystem_lookup_or_create(self->fs, q->open.path);

      log_filecell(self->log, cell);
      log_fileentry(self->log, e);
      cell->entry = e;
      if (e->seen < 0)
        record_seen(self, e, 0, q->time);

      enum accesslevel level =
        (q->tag == Q_OPRD) ? FILE_READ : FILE_WRITE;

      if (level == FILE_READ)
      {
        if (e->saved.level < FILE_READ)
        {
          if (!fs_path)
            fs_path = lookup_path(self, q->open.path, fs_path_buffer, NULL);
          if (!fs_path)
          {
            if (!e->edit_data)
              mabort("path: %s\nmode:%c\n", q->open.path, (q->tag == Q_OPRD) ? 'r' : 'w');
            e->saved.level = FILE_READ;
            memset(&e->fs_stat, 0, sizeof(e->fs_stat));
          }
          else
          {
            if (fs_path == q->open.path)
              fs_path = e->path;
            tbuf *data = tbuf_read_file(fs_path);
            if (!data)
            {
              // Pitex: unreadable file (permissions, race with deletion):
              // report as missing instead of aborting.
              log_filecell(self->log, cell);
              cell->entry = NULL;
              record_seen(self, e, INT_MAX, q->time);
              a.tag = A_PASS;
              channel_write_answer(self->c, p->fd, &a);
              break;
            }
            tbuf_drop(e->fs_data);
            e->fs_data = data;
            e->saved.level = FILE_READ;
            stat(fs_path, &e->fs_stat);
          }
        }
      }
      else
      {
        e->saved.data = tbuf_new(1024);
        e->saved.level = level;
      }

      if (level != FILE_READ)
      {
        if (strcmp(q->open.path, "stdout") == 0)
        {
          if (self->st.stdout.entry != NULL)
          {
            fprintf(stderr, "[error] two stdouts!\n");
            mabort();
          }
          log_filecell(self->log, &self->st.stdout);
          self->st.stdout.entry = e;
        }
        else
        {
          char *ext = last_index(q->open.path, '.');
          if (!ext);
          else if ((strcmp(ext, "xdv") == 0 ||
                    strcmp(ext, "dvi") == 0 ||
                    strcmp(ext, "pdf") == 0))
          {
            if (self->st.document.entry != NULL)
            {
              fprintf(stderr, "[error] two outputs!\n");
              mabort();
            }
            log_filecell(self->log, &self->st.document);
            self->st.document.entry = e;
            xdv_index_reset(self->dvi);
          }
          else if ((strcmp(ext, "synctex") == 0))
          {
            if (self->st.synctex.entry != NULL)
            {
              fprintf(stderr, "[error] two synctex!\n");
              mabort();
            }
            log_filecell(self->log, &self->st.synctex);
            self->st.synctex.entry = e;
          }
          else if ((strcmp(ext, "log") == 0))
          {
            if (self->st.log.entry != NULL)
            {
              fprintf(stderr, "[error] two log files!\n");
              mabort();
            }
            log_filecell(self->log, &self->st.log);
            self->st.log.entry = e;
            self->log_entry = e;
          }
        }
      }

      int n = strlen(q->open.path);
      a.open.path_len = n;
      a.tag = A_OPEN;
      memmove(channel_get_buffer(self->c, n), q->open.path, n);
      channel_write_answer(self->c, p->fd, &a);
      break;
    }
    case Q_READ:
    {
      check_fid(q->read.fid);
      fileentry_t *e = self->st.table[q->read.fid].entry;
      if (e == NULL) mabort();
      if (e->saved.level < FILE_READ) mabort();
      tbuf *data = entry_data(e);
      if (q->read.pos > (int)data->len)
      {
        fprintf(stderr, "read:%d\ndata->len:%d\n", q->read.pos, (int)data->len);
        mabort();
      }
      ssize_t n = q->read.size;
      if (n > (ssize_t)data->len - q->read.pos)
        n = data->len - q->read.pos;

      int fork = 0;
      if (self->fence_pos >= 0 &&
          self->fences[self->fence_pos].entry == e &&
          self->fences[self->fence_pos].position < q->read.pos + n)
      {
        if (n < 0)
          mabort();
        n = self->fences[self->fence_pos].position - q->read.pos;
        fork = (n == 0);
        if (n < 0)
          mabort("n:%d fence_pos:%d read_pos:%d\n", (int)n, self->fences[self->fence_pos].position, q->read.pos);
      }
      if (fork)
      {
        a.tag = A_FORK;
        self->fence_pos -= 1;
      }
      else if (need_snapshot(self, q->time))
      {
        a.tag = A_FORK;
      }
      else
      {
        memmove(channel_get_buffer(self->c, n), data->data + q->read.pos, n);
        a.tag = A_READ;
        a.read.size = n;
      }
      channel_write_answer(self->c, p->fd, &a);
      break;
    }
    case Q_APND:
    {
      fileentry_t *e = NULL;

      if (q->apnd.fid == -1)
      {
        e = self->st.stdout.entry;
        if (e == NULL)
        {
          e = filesystem_lookup_or_create(self->fs, "stdout");
          log_fileentry(self->log, e);
          log_filecell(self->log, &self->st.stdout);
          self->st.stdout.entry = e;
          if (e->saved.data == NULL)
          {
            e->saved.data = tbuf_new(1024);
            e->saved.level = FILE_WRITE;
          }
        }
      }
      else
      {
        check_fid(q->apnd.fid);
        e = self->st.table[q->apnd.fid].entry;
      }

      if (e == NULL || e->saved.level != FILE_WRITE) mabort();
      log_fileentry(self->log, e);

      tbuf_append(e->saved.data, q->apnd.buf, q->apnd.size);

      if (self->st.document.entry == e)
        xdv_index_update(self->dvi, e->saved.data->data, e->saved.data->len);
      else if (self->st.synctex.entry == e || self->st.log.entry == e ||
               self->st.stdout.entry == e)
        ;
      else
        self->aux_dirty = true;
      a.tag = A_DONE;
      channel_write_answer(self->c, p->fd, &a);
      break;
    }
    case Q_CLOS:
    {
      check_fid(q->clos.fid);

      filecell_t *cell = &self->st.table[q->clos.fid];
      fileentry_t *e = cell->entry;
      if (e == NULL) mabort();
      log_filecell(self->log, cell);
      cell->entry = NULL;

      if (self->st.stdout.entry == e)
      {
        log_filecell(self->log, &self->st.stdout);
        self->st.stdout.entry = NULL;
      }

      if (self->st.log.entry == e)
      {
        log_filecell(self->log, &self->st.log);
        self->st.log.entry = NULL;
      }

      a.tag = A_DONE;
      channel_write_answer(self->c, p->fd, &a);
      break;
    }
    case Q_SIZE:
    {
      check_fid(q->clos.fid);
      fileentry_t *e = self->st.table[q->clos.fid].entry;
      if (e == NULL || e->saved.level < FILE_READ) mabort();
      a.tag = A_SIZE;
      a.size.size = entry_data(e)->len;
      channel_write_answer(self->c, p->fd, &a);
      break;
    }
    case Q_MTIM:
    {
      check_fid(q->clos.fid);
      fileentry_t *e = self->st.table[q->clos.fid].entry;
      if (e == NULL || e->saved.level < FILE_READ) mabort();
      a.tag = A_MTIM;
      a.mtim.mtime = e->fs_stat.st_mtime;
      channel_write_answer(self->c, p->fd, &a);
      break;
    }
    case Q_SEEN:
    {
      check_fid(q->seen.fid);
      fileentry_t *e = self->st.table[q->seen.fid].entry;
      if (e == NULL) mabort();
      if (e->saved.level < FILE_READ) mabort();
      if (self->fence_pos >= 0 &&
          self->fences[self->fence_pos].entry == e &&
          self->fences[self->fence_pos].position < q->seen.pos)
      {
        fprintf(stderr,
                "Seen position invalid wrt fence:\n"
                "  file %s, seen: %d -> %d\n"
                "  fence #%d position: %d\n",
                e->path, e->seen, q->seen.pos,
                self->fence_pos,
                self->fences[self->fence_pos].position);
        mabort();
      }
      if (q->seen.pos > e->seen)
      {
        log_fileentry(self->log, e);
        record_seen(self, e, q->seen.pos, q->time);
      }
      break;
    }
    case Q_GPIC:
    {
      fileentry_t *e = filesystem_lookup(self->fs, q->gpic.path);
      if (e && e->saved.level == FILE_READ &&
          e->pic_cache.type == q->gpic.type &&
          e->pic_cache.page == q->gpic.page)
      {
        a.gpic.bounds[0] = e->pic_cache.bounds[0];
        a.gpic.bounds[1] = e->pic_cache.bounds[1];
        a.gpic.bounds[2] = e->pic_cache.bounds[2];
        a.gpic.bounds[3] = e->pic_cache.bounds[3];
        a.tag = A_GPIC;
      }
      else
        a.tag = A_PASS;
      channel_write_answer(self->c, p->fd, &a);
      break;
    }
    case Q_SPIC:
    {
      fileentry_t *e = filesystem_lookup(self->fs, q->spic.path);
      if (e && e->saved.level == FILE_READ)
          e->pic_cache = q->spic.cache;
      a.tag = A_DONE;
      channel_write_answer(self->c, p->fd, &a);
      break;
    }

    case Q_CHLD:
    {
      if (self->process_count == MAX_PROCESS)
      {
        decimate_processes(self);
        p = get_process(self);
      }
      channel_reset(self->c);
      self->process_count += 1;
      process_t *p2 = get_process(self);
      p->snap = log_snapshot(self->log);
      p2->fd = q->chld.fd;
      p2->pid = q->chld.pid;
      p2->trace_len = p->trace_len;
      p2->forked = true;
      fcntl(p2->fd, F_SETFD, FD_CLOEXEC);
      a.tag = A_DONE;
      channel_write_answer(self->c, p->fd, &a);
      break;
    }

    case Q_FNTB:
    {
      // Pitex: the running checkpoint child needs a new platform font; it
      // exits right after this message. Restart from a fresh engine and do
      // not checkpoint before this trace position.
      self->snapshot_barrier = maxi(self->snapshot_barrier, p->trace_len);
      self->barrier_pending = true;
      fprintf(stderr, "[process] font barrier at trace position %d\n", p->trace_len);
      break;
    }
  }
}

static void revert_trace(trace_entry_t *te)
{
  te->entry->seen = te->seen;
}

static void rollback_processes(struct tex_engine *self, int reverted, int trace)
{
  /* Pitex: keep the convergence stash. It is the aux input the surviving
   * snapshots already read, so the next pass end can detect convergence
   * without a full rerun after every edit (TeXpresso cleared it here and
   * reran from scratch on idle). */
  self->aux_dirty = false;
  self->finishing = false;

  /* A worker killed before its first trace (or at the rollback boundary)
   * is not a resumable checkpoint. Keeping it leaves prepare_process with
   * a dead top and no new worker, so every later update stays idle. */
  while (self->process_count > 0 &&
         (get_process(self)->trace_len > trace || get_process(self)->fd == -1))
    pop_process(self);

  int trace_len = self->process_count == 0 ? 0 : get_process(self)->trace_len;
  while (reverted > trace_len)
  {
    reverted--;
    revert_trace(&self->trace[reverted]);
  }

  if (self->st.document.entry)
    xdv_index_update(self->dvi, self->st.document.entry->saved.data->data,
                     self->st.document.entry->saved.data->len);
  else
    xdv_index_reset(self->dvi);
}

static bool possible_fence(trace_entry_t *te)
{
  if (te->seen == INT_MAX || te->seen == -1)
    return 0;
  if (te->entry->saved.level > FILE_READ)
    return 0;
  return 1;
}

static int compute_fences(struct tex_engine *self, int trace, int offset)
{
  self->fence_pos = -1;

  if (trace <= 0)
    return trace;

  if (get_process(self)->trace_len <= trace)
    mabort();

  self->fence_pos = 0;

  offset = (offset - 64) & ~(64 - 1);
  if (offset < self->trace[trace].seen)
    offset = self->trace[trace].seen;
  if (offset == -1)
    offset = 0;

  self->fences[0].entry = self->trace[trace].entry;
  self->fences[0].position = offset;

  int delta = 50;
  int time = self->trace[trace].time - 10;

  int target_process = self->process_count - 1;
  while (target_process >= 0 && self->processes[target_process].trace_len > trace)
    target_process -= 1;
  int target_trace = target_process >= 0 ? self->processes[target_process].trace_len : -1;
  while (trace > target_trace && self->fence_pos < 15)
  {
    if (self->trace[trace].time <= time && possible_fence(&self->trace[trace]))
    {
      self->fence_pos += 1;
      self->fences[self->fence_pos].entry = self->trace[trace].entry;
      self->fences[self->fence_pos].position = self->trace[trace].seen;
      if (self->fences[self->fence_pos].position == -1)
        self->fences[self->fence_pos].position = 0;
      time -= delta;
      delta *= 2;
    }
    trace -= 1;
  }

  return trace;
}

bool txp_engine_step(tex_engine *self, bool restart_if_needed)
{
  if (restart_if_needed)
    prepare_process(self);

  if (txp_engine_get_status(self) == DOC_RUNNING)
  {
    query_t q;
    int fd = get_process(self)->fd;
    if (fd == -1)
      return 0;
    if (!channel_has_pending_query(self->c, fd, 10))
      return 0;
    if (!read_query(self, self->c, &q))
    {
      get_process(self)->fd = -1;
      if (self->process_count == 1)
        reap_root(self, false);
      return 0;
    }
    touch_query_clock(self);
    answer_query(self, &q);
    channel_flush(self->c, fd);
    return 1;
  }

  return 0;
}

static int scan_entry(struct tex_engine *self, fileentry_t *e)
{
  if (e->saved.level < FILE_READ || e->fs_stat.st_ino == 0 || e->edit_data)
    return -1;

  struct stat st;

  char fs_path_buffer[1024];
  const char *fs_path = lookup_path(self, e->path, fs_path_buffer, &st);

  if (!fs_path)
  {
    /* Gone from disk: drop the cached bytes so reads see a real
     * file-not-found instead of silently typesetting stale content. */
    if (e->fs_data)
    {
      tbuf_drop(e->fs_data);
      e->fs_data = NULL;
      memset(&e->fs_stat, 0, sizeof e->fs_stat);
      return 0;
    }
    return -1;
  }

  if (stat_same(&st, &e->fs_stat))
    return -1;

  e->fs_stat = st;

  tbuf *buf = tbuf_read_file(fs_path);
  if (!buf)
    return -1;

  e->pic_cache.type = -1;

  int olen = e->fs_data ? (int)e->fs_data->len : 0, nlen = buf->len;
  int len = olen < nlen ? olen : nlen;

  int i = 0;
  while (i < len && e->fs_data->data[i] == buf->data[i])
    i += 1;

  if (i == len && olen == nlen)
  {
    tbuf_drop(buf);
    return -1;
  }

  fprintf(stderr, "[scan] %s changed at byte %d\n", e->path, i);
  tbuf_drop(e->fs_data);
  e->fs_data = buf;

  return i;
}

#define NOT_IN_TRANSACTION (-2)

static void rollback_begin(struct tex_engine *self)
{
  // Check if already in a transaction
  if (self->rollback.trace_len != NOT_IN_TRANSACTION)
    abort();

  // Skip if no worker yet
  if (self->process_count == 0)
    return;

  self->rollback.trace_len = get_process(self)->trace_len;
  self->rollback.offset = -1;
  self->rollback.flush = 0;
}

static bool rollback_end(struct tex_engine *self, int *tracep, int *offsetp)
{
  int trace_len = self->rollback.trace_len;
  self->rollback.trace_len = NOT_IN_TRANSACTION;

  // No transaction opened: legitimate when begin was a no-op (no worker).
  if (trace_len == NOT_IN_TRANSACTION)
    return false;

  process_t *p = get_process(self);

  // Check if nothing changed
  if (trace_len == p->trace_len)
  {
    if (!self->rollback.flush)
      return false;
    if (p->fd > -1)
    {
      ask_t a;
      a.tag = C_FLSH;
      channel_write_ask(self->c, p->fd, &a);
      channel_flush(self->c, p->fd);
      return false;
    }
    if (trace_len > 0)
    {
      trace_len -= 1;
      revert_trace(&self->trace[trace_len]);
    }
    if (trace_len > 0)
      self->rollback.offset = self->trace[trace_len].seen;
  }

  if (tracep)
    *tracep = trace_len;
  if (offsetp)
    *offsetp = self->rollback.offset;

  return true;
}

// Return false if some contents had not been observed: caller should recheck
// for changed contents.
// Return true otherwise (process is ready to be flushed).
static bool process_pending_messages(struct tex_engine *self)
{
  // If the process is marked ready to flush, seen messages have already been
  // consumed
  if (self->rollback.flush)
    return 1;

  process_t *p = get_process(self);

  // If process is dead, nothing has been missed
  if (p->fd == -1)
    return 1;

  // Synchronize with the child process:
  // - kill if stuck
  // - check pending SEEN messages to update vision of the process
  int nothing_seen = 1;
  do {
    if (!channel_has_pending_query(self->c, p->fd, 10))
    {
      // The process hasn't answered in 10ms: it might be stuck in a long
      // computation or a loop, kill it to start from the previous one.
      fprintf(stderr, "[kill] worker might be stuck, killing\n");
      close_process(p);
      break;
    }
    // Process only pending SEEN to have an updated view on process state
    switch (channel_peek_query(self->c, p->fd))
    {
      case Q_SEEN:
        {
          query_t q;
          if (!read_query(self, self->c, &q))
          {
            p->fd = -1;
            break;
          }
          answer_query(self, &q);
          nothing_seen = 0;
          continue;
        }
      default:
        break;
    }
  } while(0);

  self->rollback.flush = 1;
  return nothing_seen;
}

static void rollback_add_change(struct tex_engine *self, fileentry_t *e, int changed)
{
  int trace_len = self->rollback.trace_len;

  // No transaction opened: legitimate when begin was a no-op (no worker).
  if (trace_len == NOT_IN_TRANSACTION)
    return;

  if (e->seen < changed && trace_len == get_process(self)->trace_len)
  {
    // A pending message might update e->seen
    if (process_pending_messages(self))
      return;
    trace_len = self->rollback.trace_len = get_process(self)->trace_len;
  }
  if (e->seen < changed)
    return;

  while (e->seen >= changed)
  {
    if (trace_len <= 0)
      mabort("rollback walked past the first trace entry for %s\n", e->path);
    trace_len--;
    revert_trace(&self->trace[trace_len]);
  }

  if (self->trace[trace_len].entry != e)
  {
    fprintf(stderr, "Rollback position: %d. Entries: %d. Seen: %d. Changed: %d.\n",
            trace_len, get_process(self)->trace_len, e->seen, changed);
    mabort();
  }

  self->rollback.trace_len = trace_len;
  self->rollback.offset = changed;
}

void txp_engine_notify_file_changes(tex_engine *self, fileentry_t *entry, int offset)
{
  rollback_add_change(self, entry, offset);
}

static bool is_system_output(const char *path)
{
  if (strcmp(path, "stdout") == 0)
    return true;
  const char *dot = strrchr(path, '.');
  if (!dot)
    return false;
  return strcmp(dot, ".log") == 0
      || strcmp(dot, ".xdv") == 0
      || strcmp(dot, ".dvi") == 0
      || strcmp(dot, ".pdf") == 0
      || strcmp(dot, ".synctex") == 0;
}

// Reset per-entry `seen` counters before respawning the engine. Without
// this, the new engine's Q_OPRD never enters the `e->seen < 0` branch
// in answer_query, so no first-open trace entry is generated. Later
// rollbacks would walk back through the trace without ever bringing
// entry->seen below the changed offset.
static void reset_seen_for_respawn(struct tex_engine *self)
{
  fileentry_t *e;
  for (int index = 0; (e = filesystem_scan(self->fs, &index));)
    e->seen = -1;
}

bool txp_engine_aux_dirty(tex_engine *self)
{
  return self->aux_dirty;
}

bool txp_engine_is_finishing(tex_engine *self)
{
  return self->finishing;
}

void txp_engine_start_finishing(tex_engine *self)
{
  self->finishing = true;
}

static void respawn_from_scratch(struct tex_engine *self)
{
  while (self->process_count > 0)
    pop_process(self);
  reset_seen_for_respawn(self);
  xdv_index_reset(self->dvi);
  self->fence_pos = -1;
  prepare_process(self);
}

bool txp_engine_finish_convergence(tex_engine *self)
{
  if (!self->finishing)
    return false;
  if (txp_engine_get_status(self) != DOC_TERMINATED)
    return false;

  self->finishing = false;

  bool converged = true;
  fileentry_t *e;
  for (int index = 0; (e = filesystem_scan(self->fs, &index));)
  {
    if (e->saved.level != FILE_WRITE || !e->saved.data)
      continue;
    if (is_system_output(e->path))
      continue;
    bool stashed_match = e->edit_data && e->edit_data_from_convergence
                         && e->saved.data->len == e->edit_data->len
                         && memcmp(e->saved.data->data, e->edit_data->data,
                                   e->saved.data->len) == 0;
    /* Pitex: output identical to the on-disk file the pass read (e.g. the
     * .aux of the last final build) is converged as well. */
    if (!stashed_match && !e->edit_data && e->fs_data)
      stashed_match = e->saved.data->len == e->fs_data->len
                      && memcmp(e->saved.data->data, e->fs_data->data,
                                e->saved.data->len) == 0;
    if (!stashed_match)
    {
      converged = false;
      break;
    }
  }

  if (converged)
  {
    fprintf(stderr, "[rerun] aux byte-stable, convergence reached\n");
    // Pitex: keep the dead top process. The driver always runs passes to
    // the end (TeXpresso only advanced to the displayed page), so popping
    // it would resume the snapshot below and typeset the tail again; the
    // next edit's rollback removes it (rollback_end handles a dead top).
    self->aux_dirty = false;
    return false;
  }

  for (int index = 0; (e = filesystem_scan(self->fs, &index));)
  {
    if (e->saved.level != FILE_WRITE || !e->saved.data)
      continue;
    if (is_system_output(e->path))
      continue;
    // Don't clobber edit_data the editor pushed
    if (e->edit_data && !e->edit_data_from_convergence)
      continue;
    tbuf_drop(e->edit_data);
    // Copy, don't share: log_rollback truncates saved.data->len in place.
    e->edit_data = tbuf_from_copy(e->saved.data->data, e->saved.data->len);
    e->edit_data_from_convergence = true;
  }

  respawn_from_scratch(self);
  self->aux_dirty = false;
  return true;
}

void txp_engine_begin_changes(tex_engine *self)
{
  rollback_begin(self);
}

void txp_engine_detect_changes(tex_engine *self)
{
  fileentry_t *e;
  for (int index = 0; (e = filesystem_scan(self->fs, &index));)
  {
    int changed = scan_entry(self, e);
    if (changed > -1)
      rollback_add_change(self, e, changed);
  }
}

/* Refresh one entry's disk copy even while an editor override exists
 * (close): detect_changes skipped it, so fs_data can predate the save
 * that made the buffer clean. No change is reported; the caller diffs
 * against the text the pass actually read. */
void txp_engine_scan_file(tex_engine *self, fileentry_t *e)
{
  if (e->saved.level < FILE_READ || e->fs_stat.st_ino == 0)
    return;
  tbuf *edit = e->edit_data;
  e->edit_data = NULL; /* scan_entry skips overridden entries */
  scan_entry(self, e);
  e->edit_data = edit;
}

bool txp_engine_end_changes(tex_engine *self)
{
  int reverted, trace, offset;

  if (!rollback_end(self, &reverted, &offset))
    return false;

  trace = reverted >= 0 ? compute_fences(self, reverted, offset) : 0;
  rollback_processes(self, reverted, trace);

  return true;
}

txp_engine_status txp_engine_get_status(tex_engine *self)
{
  if (self->process_count == 0)
    return DOC_TERMINATED;
  return get_process(self)->fd > -1 ? DOC_RUNNING : DOC_TERMINATED;
}

fileentry_t *txp_engine_find_file(tex_engine *self, const char *path)
{
  return filesystem_lookup_or_create(self->fs, path);
}

tex_engine *txp_engine_new(const char *engine_path,
                           const char *inclusion_path, const char *tex_name)
{
  struct tex_engine *self = calloc(1, sizeof(struct tex_engine));
  if (!self)
    abort();

  self->name = strdup(tex_name);
  self->engine_path = strdup(engine_path);
  self->inclusion_path = strdup(inclusion_path ? inclusion_path : "");
  state_init(&self->st);
  self->fs = filesystem_new();
  self->log = log_new();
  self->trace = NULL;
  self->trace_cap = 0;
  self->fence_pos = -1;
  self->restart = log_snapshot(self->log);
  self->c = channel_new();
  self->process_count = 0;

  self->dvi = xdv_index_new();
  self->rollback.trace_len = NOT_IN_TRANSACTION;
  self->last_status = -1;
  self->snapshot_barrier = -1;

  return self;
}

/* Pitex additions */

xdv_index *txp_engine_xdv(tex_engine *self) { return self->dvi; }

tbuf *txp_engine_document(tex_engine *self)
{
  return self->st.document.entry ? self->st.document.entry->saved.data : NULL;
}

tbuf *txp_engine_synctex(tex_engine *self)
{
  return self->st.synctex.entry ? self->st.synctex.entry->saved.data : NULL;
}

tbuf *txp_engine_log(tex_engine *self)
{
  return self->log_entry ? self->log_entry->saved.data : NULL;
}

filesystem_t *txp_engine_fs(tex_engine *self) { return self->fs; }

int txp_engine_fd(tex_engine *self)
{
  if (self->process_count == 0)
    return -1;
  return get_process(self)->fd;
}

int txp_engine_idle_ms(tex_engine *self)
{
  struct timespec now;
  clock_gettime(CLOCK_MONOTONIC, &now);
  long ms = (now.tv_sec - self->last_query.tv_sec) * 1000 +
            (now.tv_nsec - self->last_query.tv_nsec) / 1000000;
  return ms > INT_MAX ? INT_MAX : (int)ms;
}

void txp_engine_kill_running(tex_engine *self)
{
  if (self->process_count == 0)
    return;
  close_process(get_process(self));
  if (self->process_count == 1)
    reap_root(self, true);
}

void txp_engine_restart(tex_engine *self)
{
  clear_convergence_stash(self);
  respawn_from_scratch(self);
}

bool txp_engine_running_forked(tex_engine *self)
{
  return self->process_count > 0 && get_process(self)->forked;
}

int txp_engine_last_status(tex_engine *self) { return self->last_status; }

/* Returns and clears the pending font-barrier restart request. */
bool txp_engine_take_barrier(tex_engine *self)
{
  bool b = self->barrier_pending;
  self->barrier_pending = false;
  return b;
}
