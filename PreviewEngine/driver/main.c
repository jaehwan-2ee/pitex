/* Pitex embedded preview engine — session driver.
 *
 * Long-lived helper spawned by Pitex per workspace preview session. Reads
 * JSON-line requests on stdin (buffer updates, artifact releases), drives
 * the TeXpresso-derived checkpointing XeTeX engine, converts its XDV output
 * to PDF with the Pitex writer and publishes preview artifacts atomically
 * into per-publication directories under --out. See PreviewEngine/PROTOCOL.md.
 *
 * The update/close handling follows TeXpresso's frontend (interpret_open /
 * interpret_close in src/frontend/main.c, MIT, Frédéric Bour); everything
 * else here is Pitex-authored (PolyForm Shield 1.0.0). */
#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>
#ifdef __linux__
#include <sys/prctl.h>
#endif
#ifdef __APPLE__
#include <mach-o/dyld.h>
#endif

#include "engine_tex.h"
#include "json.h"
#include "providers.h"
#include "xdv2pdf.h"

#define MAX_RERUNS 5
#define MAX_UNRELEASED 4
#define INTERMEDIATE_INTERVAL_MS 350
#define WATCHDOG_MS 20000
#define LOG_CAP (8 << 20)

/* ------------------------------------------------------------------ */
/* Configuration                                                       */

static struct {
  char root[PATH_MAX];
  char main_dir[PATH_MAX];
  char job[256];      /* main file name without extension */
  char tex_name[256]; /* main file name as passed to the engine */
  char out[PATH_MAX];
  char engine[PATH_MAX];
} cfg;

static int log_fd = -1;

static long long now_ms(void)
{
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (long long)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

static void emit(pbuf *line)
{
  pbuf_putc(line, '\n');
  size_t off = 0;
  while (off < line->len)
  {
    ssize_t n = write(STDOUT_FILENO, line->data + off, line->len - off);
    if (n < 0)
    {
      if (errno == EINTR)
        continue;
      _exit(0); /* client went away */
    }
    off += (size_t)n;
  }
}

static void emit_simple(const char *event, const char *code, const char *message)
{
  pbuf b = { 0 };
  pbuf_printf(&b, "{\"event\":\"%s\"", event);
  if (code)
    pbuf_printf(&b, ",\"code\":\"%s\"", code);
  if (message)
  {
    pbuf_puts(&b, ",\"message\":");
    json_write_string(&b, message, strlen(message));
  }
  pbuf_putc(&b, '}');
  emit(&b);
  pbuf_free(&b);
}

/* ------------------------------------------------------------------ */
/* Resource resolution for the PDF writer                              */

typedef struct res_entry {
  struct res_entry *next;
  char *name;
  int kind;
  char *path;
  struct stat st;
  tbuf *data;
} res_entry;

static tex_engine *engine;
static res_entry *res_cache;

static bool stat_equal(const struct stat *a, const struct stat *b)
{
  return a->st_ino == b->st_ino && a->st_dev == b->st_dev && a->st_size == b->st_size &&
         a->st_mtime == b->st_mtime;
}

static const char *const exts_font[] = { "", ".otf", ".ttf", ".ttc", ".OTF", ".TTF", ".pfb", NULL };
static const char *const exts_tfm[] = { ".tfm", "", NULL };
static const char *const exts_vf[] = { ".vf", NULL };
static const char *const exts_enc[] = { "", ".enc", NULL };
static const char *const exts_plain[] = { "", NULL };
static const char *const exts_type1[] = { "", ".pfb", ".pfa", NULL };

static const char *find_on_disk(const char *name, const char *const *exts, char buf[PATH_MAX])
{
  struct stat st;
  for (int i = 0; exts[i]; i++)
  {
    if (snprintf(buf, PATH_MAX, "%s%s", name, exts[i]) >= PATH_MAX)
      continue;
    if (stat(buf, &st) == 0 && S_ISREG(st.st_mode))
      return buf;
  }
  if (name[0] == '/')
    return NULL;
  for (int i = 0; exts[i]; i++)
  {
    char tmp[PATH_MAX];
    if (snprintf(tmp, sizeof tmp, "%s%s", name, exts[i]) >= (int)sizeof tmp)
      continue;
    const char *p = texlive_file_path(tmp, NULL);
    if (p && stat(p, &st) == 0 && S_ISREG(st.st_mode))
    {
      snprintf(buf, PATH_MAX, "%s", p);
      return buf;
    }
  }
  return NULL;
}

static tbuf *resolve(void *env, const char *name, xdv_res_kind kind)
{
  (void)env;
  /* Files the engine read through the VFS (edited buffers, project images)
   * resolve to exactly the bytes TeX saw. */
  if (engine && (kind == RES_IMAGE || kind == RES_NATIVE_FONT))
  {
    fileentry_t *e = filesystem_lookup(txp_engine_fs(engine), name);
    if (e)
    {
      tbuf *d = e->edit_data ? e->edit_data : e->fs_data;
      if (d)
        return tbuf_keep(d);
    }
  }
  const char *const *exts = exts_plain;
  switch (kind)
  {
    case RES_NATIVE_FONT: exts = exts_font; break;
    case RES_TFM: exts = exts_tfm; break;
    case RES_VF: exts = exts_vf; break;
    case RES_ENC: exts = exts_enc; break;
    case RES_TYPE1: exts = exts_type1; break;
    default: break;
  }
  res_entry *r;
  for (r = res_cache; r; r = r->next)
    if (r->kind == (int)kind && strcmp(r->name, name) == 0)
      break;
  if (r && r->path)
  {
    struct stat st;
    if (stat(r->path, &st) == 0 && stat_equal(&st, &r->st))
      return tbuf_keep(r->data);
  }
  char buf[PATH_MAX];
  const char *path = find_on_disk(name, exts, buf);
  tbuf *data = path ? tbuf_read_file(path) : NULL;
  if (!r)
  {
    r = calloc(1, sizeof *r);
    if (!r)
      abort();
    r->name = strdup(name);
    r->kind = (int)kind;
    r->next = res_cache;
    res_cache = r;
  }
  free(r->path);
  tbuf_drop(r->data);
  r->path = NULL;
  r->data = NULL;
  if (data)
  {
    r->path = strdup(path);
    stat(path, &r->st);
    r->data = data;
    return tbuf_keep(data);
  }
  return NULL;
}

/* ------------------------------------------------------------------ */
/* Pass snapshots and publication                                      */

typedef struct {
  tbuf *xdv, *synctex, *log;
  xdv_index *index;
} snapshot;

static void snapshot_clear(snapshot *s)
{
  tbuf_drop(s->xdv);
  tbuf_drop(s->synctex);
  tbuf_drop(s->log);
  xdv_index_free(s->index);
  memset(s, 0, sizeof *s);
}

static tbuf *copy_or_empty(tbuf *b) { return b ? tbuf_from_copy(b->data, b->len) : tbuf_new(16); }

static void snapshot_take(snapshot *s)
{
  snapshot_clear(s);
  s->xdv = copy_or_empty(txp_engine_document(engine));
  s->synctex = copy_or_empty(txp_engine_synctex(engine));
  s->log = copy_or_empty(txp_engine_log(engine));
  s->index = xdv_index_new();
  xdv_index_update(s->index, s->xdv->data, s->xdv->len);
}

static snapshot last_complete;    /* last normally-finished pass */
static snapshot pending;          /* complete publication deferred by backpressure */
static bool pending_valid, pending_complete;
static long long pending_generation;

static xdv2pdf *writer;
static int publish_seq;
/* Complete publication that reflects the sources of the newest update;
 * 0 after a failure or while a pass runs (reported by "idle"). */
static int current_seq;
static long long generation;      /* newest applied update */
static long long generation_start_ms;
static long long last_publish_ms;
static int published_pages_this_pass;
static int rerun_count;
static bool crash_retried, watchdog_killed, was_running;

typedef struct {
  int seq;
  bool used;
} artifact;
static artifact artifacts[MAX_UNRELEASED];

static int unreleased_count(void)
{
  int n = 0;
  for (int i = 0; i < MAX_UNRELEASED; i++)
    n += artifacts[i].used;
  return n;
}

static bool contains(const tbuf *b, const char *needle)
{
  if (!b)
    return false;
  size_t n = strlen(needle);
  for (size_t i = 0; i + n <= b->len; i++)
    if (b->data[i] == (unsigned char)needle[0] && memcmp(b->data + i, needle, n) == 0)
      return true;
  return false;
}

/* First TeX error ("! message" + "l.<n>") and error count from a log. */
static int log_errors(const tbuf *log, char *first, size_t cap)
{
  int count = 0;
  first[0] = 0;
  if (!log)
    return 0;
  const char *s = (const char *)log->data, *end = s + log->len;
  const char *line = s;
  while (line < end)
  {
    const char *nl = memchr(line, '\n', (size_t)(end - line));
    if (!nl)
      nl = end;
    if (nl - line >= 2 && line[0] == '!' && line[1] == ' ')
    {
      if (count == 0)
      {
        int n = (int)(nl - line - 2);
        if (n > (int)cap - 32)
          n = (int)cap - 32;
        snprintf(first, cap, "%.*s", n, line + 2);
        /* look ahead for "l.<n>" */
        const char *q = nl;
        for (int k = 0; k < 12 && q < end; k++)
        {
          const char *qn = memchr(q + 1, '\n', (size_t)(end - q - 1));
          if (!qn)
            qn = end;
          const char *ql = q + 1;
          if (qn - ql > 2 && ql[0] == 'l' && ql[1] == '.' && ql[2] >= '0' && ql[2] <= '9')
          {
            size_t used = strlen(first);
            snprintf(first + used, cap - used, " (line %ld)", strtol(ql + 2, NULL, 10));
            break;
          }
          q = qn;
        }
      }
      count++;
    }
    line = nl + 1;
  }
  return count;
}

/* Rewrite relative SyncTeX Input paths to absolute ones (relative to the
 * directory the engine ran in). The saved buffer may end mid-sheet or
 * carry sheets past `pages` (a partial pass): emit only complete
 * `{N … }N` sheet records with N <= pages, then always close the file
 * with a well-formed postamble whose Count is the emitted sheet count. */
static void write_synctex(FILE *f, const tbuf *sx, int pages)
{
  if (!sx || sx->len == 0)
    return;
  const char *s = (const char *)sx->data, *end = s + sx->len;
  /* First pass: find the cut point and count complete sheets. A line
   * without a terminating newline is never complete; a sheet numbered
   * past `pages` is cut at its opening `{N`. The engine's own postamble
   * survives only when it lies before the cut. */
  const char *cut = end, *sheet_at = NULL, *post_at = NULL;
  int sheets_done = 0;
  for (const char *line = s; line < end;)
  {
    const char *nl = memchr(line, '\n', (size_t)(end - line));
    if (!nl)
    {
      cut = sheet_at && sheet_at < line ? sheet_at : line;
      break; /* cut a mid-line record */
    }
    size_t n = (size_t)(nl - line);
    if (n >= 10 && memcmp(line, "Postamble:", 10) == 0)
      post_at = line;
    if (n > 1 && line[0] == '{')
      sheet_at = line; /* sheet N record opens */
    else if (n > 0 && line[0] == '}')
    {
      unsigned num = 0;
      bool numok = false;
      for (size_t i = 1; i < n && line[i] >= '0' && line[i] <= '9'; i++)
      {
        num = num * 10 + (unsigned)(line[i] - '0');
        numok = true;
      }
      if (numok && (int)num <= pages)
        sheets_done++;
      else
      {
        cut = sheet_at && sheet_at < line ? sheet_at : line;
        break; /* sheets beyond `pages` never emit */
      }
      sheet_at = NULL;
    }
    line = nl + 1;
  }
  if (sheet_at && sheet_at < cut)
    cut = sheet_at; /* EOF inside a sheet record */
  const bool post_emitted = post_at && post_at < cut;
  /* Second pass: emit lines up to the cut, rewriting Input paths. */
  for (const char *line = s; line < cut;)
  {
    const char *nl = memchr(line, '\n', (size_t)(cut - line));
    size_t n = (size_t)((nl ? nl : cut) - line);
    if (n > 6 && memcmp(line, "Input:", 6) == 0)
    {
      const char *colon = memchr(line + 6, ':', n - 6);
      if (colon && colon + 1 < line + n && colon[1] != '/')
      {
        const char *path = colon + 1;
        if (path + 2 <= line + n && path[0] == '.' && path[1] == '/')
          path += 2;
        fwrite(line, 1, (size_t)(colon + 1 - line), f);
        fprintf(f, "%s/%.*s\n", cfg.main_dir, (int)(line + n - path), path);
        line += n + 1;
        continue;
      }
    }
    fwrite(line, 1, n, f);
    fputc('\n', f);
    line += n + 1;
  }
  if (!post_emitted)
    fprintf(f, "Postamble:\nCount:%d\nPost scriptum:\n", sheets_done);
}

static bool write_file(const char *dir, const char *name, const void *data, size_t len)
{
  char path[PATH_MAX];
  snprintf(path, sizeof path, "%s/%s", dir, name);
  int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
  if (fd < 0)
    return false;
  const unsigned char *p = data;
  while (len > 0)
  {
    ssize_t n = write(fd, p, len);
    if (n < 0)
    {
      if (errno == EINTR)
        continue;
      close(fd);
      return false;
    }
    p += n;
    len -= (size_t)n;
  }
  return close(fd) == 0;
}

static void remove_dir(const char *dir)
{
  DIR *d = opendir(dir);
  if (d)
  {
    struct dirent *de;
    while ((de = readdir(d)))
    {
      if (!strcmp(de->d_name, ".") || !strcmp(de->d_name, ".."))
        continue;
      char p[PATH_MAX];
      snprintf(p, sizeof p, "%s/%s", dir, de->d_name);
      unlink(p);
    }
    closedir(d);
  }
  rmdir(dir);
}

static void cap_log_file(void)
{
  struct stat st;
  if (log_fd >= 0 && fstat(log_fd, &st) == 0 && st.st_size > LOG_CAP)
    if (ftruncate(log_fd, 0) != 0)
      ;
}

/* Publish `cur` pages, completed by the stale tail of `last_complete` for
 * intermediate snapshots. Returns false when backpressure blocks it. */
static bool publish(const snapshot *cur, bool complete, long long gen)
{
  if (unreleased_count() >= MAX_UNRELEASED)
    return false;
  int slot = -1;
  for (int i = 0; i < MAX_UNRELEASED; i++)
    if (!artifacts[i].used)
    {
      slot = i;
      break;
    }

  long long t0 = now_ms();
  int ncur = xdv_index_page_count(cur->index);
  xdv_range ranges[2];
  int nranges = 0;
  if (ncur > 0)
    ranges[nranges++] = (xdv_range){ cur->xdv->data, cur->xdv->len, cur->index, 0, ncur };
  int total = ncur;
  if (!complete && last_complete.index && cur != &last_complete)
  {
    int nold = xdv_index_page_count(last_complete.index);
    if (nold > ncur)
    {
      ranges[nranges++] = (xdv_range){ last_complete.xdv->data, last_complete.xdv->len, last_complete.index,
                                       ncur, nold - ncur };
      total = nold;
    }
  }
  if (total == 0)
    return true;
  /* SyncTeX only when every page comes from this pass: a stale tail from
   * the previous pass would be misattributed. */
  const bool with_synctex = total == ncur && cur->synctex && cur->synctex->len;

  pbuf pdf = { 0 }, warnings = { 0 };
  int err = xdv2pdf_write(writer, ranges, nranges, &pdf, &warnings);
  if (warnings.len)
    fprintf(stderr, "[pdf] warnings:\n%.*s", (int)warnings.len, (char *)warnings.data);
  if (err)
  {
    char msg[256];
    snprintf(msg, sizeof msg, "PDF conversion failed (%d)", err);
    emit_simple("error", "pdf", msg);
    pbuf_free(&pdf);
    pbuf_free(&warnings);
    return true;
  }

  int seq = ++publish_seq;
  char tmp[PATH_MAX], dir[PATH_MAX], name[300];
  snprintf(tmp, sizeof tmp, "%s/.p%d.tmp", cfg.out, seq);
  snprintf(dir, sizeof dir, "%s/p%d", cfg.out, seq);
  bool ok = mkdir(tmp, 0700) == 0;
  snprintf(name, sizeof name, "%s.pdf", cfg.job);
  ok = ok && write_file(tmp, name, pdf.data, pdf.len);
  if (ok && with_synctex)
  {
    char p[PATH_MAX];
    snprintf(p, sizeof p, "%s/%s.synctex", tmp, cfg.job);
    FILE *f = fopen(p, "wb");
    ok = f != NULL;
    if (f)
    {
      write_synctex(f, cur->synctex, ncur);
      ok = fclose(f) == 0;
    }
  }
  snprintf(name, sizeof name, "%s.log", cfg.job);
  ok = ok && write_file(tmp, name, cur->log ? cur->log->data : (const unsigned char *)"", cur->log ? cur->log->len : 0);
  ok = ok && rename(tmp, dir) == 0;
  pbuf_free(&pdf);
  if (!ok)
  {
    remove_dir(tmp);
    emit_simple("error", "io", strerror(errno));
    pbuf_free(&warnings);
    return true;
  }
  artifacts[slot].used = true;
  artifacts[slot].seq = seq;

  char first[512];
  int errors = log_errors(cur->log, first, sizeof first);
  pbuf ev = { 0 };
  pbuf_printf(&ev, "{\"event\":\"published\",\"seq\":%d,\"generation\":%lld,\"complete\":%s,\"pages\":%d,"
                   "\"current_pages\":%d,\"errors\":%d,\"elapsed_ms\":%lld,\"pdf_ms\":%lld,\"dir\":",
              seq, gen, complete ? "true" : "false", total, ncur, errors, now_ms() - generation_start_ms,
              now_ms() - t0);
  json_write_string(&ev, dir, strlen(dir));
  char p[PATH_MAX];
  snprintf(p, sizeof p, "%s/%s.pdf", dir, cfg.job);
  pbuf_puts(&ev, ",\"pdf\":");
  json_write_string(&ev, p, strlen(p));
  pbuf_printf(&ev, ",\"coherent\":%s", total == ncur ? "true" : "false");
  if (with_synctex)
  {
    snprintf(p, sizeof p, "%s/%s.synctex", dir, cfg.job);
    pbuf_puts(&ev, ",\"synctex\":");
    json_write_string(&ev, p, strlen(p));
  }
  snprintf(p, sizeof p, "%s/%s.log", dir, cfg.job);
  pbuf_puts(&ev, ",\"log\":");
  json_write_string(&ev, p, strlen(p));
  if (errors)
  {
    pbuf_puts(&ev, ",\"first_error\":");
    json_write_string(&ev, first, strlen(first));
  }
  if (warnings.len)
  {
    pbuf_puts(&ev, ",\"warnings\":");
    json_write_string(&ev, (char *)warnings.data, warnings.len > 4096 ? 4096 : warnings.len);
  }
  pbuf_putc(&ev, '}');
  emit(&ev);
  pbuf_free(&ev);
  pbuf_free(&warnings);
  last_publish_ms = now_ms();
  current_seq = complete ? seq : 0;
  cap_log_file();
  return true;
}

static void release(int seq)
{
  for (int i = 0; i < MAX_UNRELEASED; i++)
  {
    if (artifacts[i].used && artifacts[i].seq == seq)
    {
      char dir[PATH_MAX];
      snprintf(dir, sizeof dir, "%s/p%d", cfg.out, seq);
      remove_dir(dir);
      artifacts[i].used = false;
    }
  }
  if (pending_valid && publish(&pending, pending_complete, pending_generation))
  {
    pending_valid = false;
    snapshot_clear(&pending);
  }
}

static void emit_failed(const tbuf *log, const char *code, const char *fallback)
{
  char first[512];
  int errors = log_errors(log, first, sizeof first);
  current_seq = 0;
  pbuf b = { 0 };
  pbuf_printf(&b, "{\"event\":\"failed\",\"generation\":%lld,\"code\":\"%s\",\"errors\":%d,\"message\":", generation,
              code, errors);
  const char *m = errors ? first : fallback;
  json_write_string(&b, m, strlen(m));
  if (log && log->len)
  {
    size_t n = log->len > 8192 ? 8192 : log->len;
    pbuf_puts(&b, ",\"log_tail\":");
    json_write_string(&b, (const char *)log->data + log->len - n, n);
  }
  pbuf_putc(&b, '}');
  emit(&b);
  pbuf_free(&b);
}

/* ------------------------------------------------------------------ */
/* Updates (TeXpresso interpret_open / interpret_close semantics)      */

static int find_diff(const tbuf *buf, const void *data, size_t size)
{
  const unsigned char *ptr = data;
  size_t len = buf->len < size ? buf->len : size;
  size_t i;
  for (i = 0; i < len && buf->data[i] == ptr[i]; ++i)
    ;
  return (int)i;
}

/* Engine-visible name for an absolute path: relative to the main file's
 * directory when inside the project (TeX resolves \input relative to it). */
static bool vfs_key(const char *abs, char out[PATH_MAX])
{
  size_t rl = strlen(cfg.root);
  if (strncmp(abs, cfg.root, rl) != 0 || (abs[rl] != '/' && abs[rl] != 0))
  {
    snprintf(out, PATH_MAX, "%s", abs);
    return true;
  }
  size_t ml = strlen(cfg.main_dir);
  if (strncmp(abs, cfg.main_dir, ml) == 0 && abs[ml] == '/')
  {
    snprintf(out, PATH_MAX, "%s", abs + ml + 1);
    return true;
  }
  /* inside root, outside main dir: "../" chain */
  const char *a = abs, *m = cfg.main_dir;
  size_t common = 0;
  for (size_t i = 0; a[i] && m[i] && a[i] == m[i]; i++)
    if (a[i] == '/')
      common = i;
  if (m[strlen(m)] == 0 && strncmp(a, m, strlen(m)) == 0)
    common = strlen(m);
  out[0] = 0;
  for (const char *q = m + common; *q; q++)
    if (*q == '/')
      strncat(out, "../", PATH_MAX - strlen(out) - 1);
  strncat(out, a + common + 1, PATH_MAX - strlen(out) - 1);
  return true;
}

/* Canonical absolute form of a client path: realpath when it exists, else
 * canonicalized parent + leaf (new buffers, deleted files). Without this
 * a client path through a symlinked /tmp or /var never matches the
 * realpath'd root, and the override lands on a file TeX never reads. */
static void canonical_path(const char *abs, char out[PATH_MAX])
{
  if (realpath(abs, out))
    return;
  snprintf(out, PATH_MAX, "%s", abs);
  char *slash = strrchr(out, '/');
  if (!slash)
    return;
  char leaf[PATH_MAX];
  snprintf(leaf, sizeof leaf, "%s", slash + 1);
  *slash = 0;
  char parent[PATH_MAX];
  if (!realpath(out[0] ? out : "/", parent))
  {
    *slash = '/';
    return;
  }
  snprintf(out, PATH_MAX, "%s/%s", parent, leaf);
}


static void interpret_open(const char *key, const void *data, size_t size)
{
  fileentry_t *e = txp_engine_find_file(engine, key);
  int changed = -1;
  if (e->edit_data && !e->edit_data_from_convergence)
  {
    changed = find_diff(e->edit_data, data, size);
    if (changed == (int)size && e->edit_data->len == size)
      return; /* unchanged */
    tbuf_drop(e->edit_data);
    e->edit_data = tbuf_from_copy(data, size);
  }
  else
  {
    tbuf_drop(e->edit_data);
    e->edit_data = tbuf_from_copy(data, size);
    e->edit_data_from_convergence = false;
    if (e->fs_data)
    {
      changed = find_diff(e->fs_data, data, size);
      if (changed == (int)size && e->fs_data->len == size)
        changed = -1;
    }
    else if (e->seen >= 0)
      changed = 0;
  }
  if (changed >= 0)
    txp_engine_notify_file_changes(engine, e, changed);
}

static void interpret_close(const char *key)
{
  fileentry_t *e = filesystem_lookup(txp_engine_fs(engine), key);
  if (!e || !e->edit_data || e->edit_data_from_convergence)
    return;
  /* The editor's override hid on-disk changes from detect_changes (the
   * coalesced save just landed). Re-read the file while the override is
   * still known, diff the saved bytes against what the pass read, then
   * drop the override — a save+close of unchanged text ends truly idle. */
  txp_engine_scan_file(engine, e);
  int changed = 0;
  if (e->fs_data)
  {
    changed = find_diff(e->fs_data, e->edit_data->data, e->edit_data->len);
    if (changed == (int)e->edit_data->len && e->fs_data->len == e->edit_data->len)
      changed = -1;
  }
  tbuf_drop(e->edit_data);
  e->edit_data = NULL;
  if (changed >= 0)
    txp_engine_notify_file_changes(engine, e, changed);
}

/* Merged pending update: newest text per file, closes not re-opened. */
typedef struct {
  bool valid;
  long long generation;
  int n, cap;
  char **paths;
  char **texts;
  size_t *lens; /* SIZE_MAX marks a close */
} merged_update;

static merged_update upd;

static void upd_set(const char *path, const char *text, size_t len)
{
  for (int i = 0; i < upd.n; i++)
    if (strcmp(upd.paths[i], path) == 0)
    {
      free(upd.texts[i]);
      upd.texts[i] = text ? malloc(len + 1) : NULL;
      if (text)
        memcpy(upd.texts[i], text, len);
      upd.lens[i] = text ? len : SIZE_MAX;
      return;
    }
  if (upd.n == upd.cap)
  {
    upd.cap = upd.cap ? upd.cap * 2 : 8;
    upd.paths = realloc(upd.paths, sizeof(char *) * upd.cap);
    upd.texts = realloc(upd.texts, sizeof(char *) * upd.cap);
    upd.lens = realloc(upd.lens, sizeof(size_t) * upd.cap);
    if (!upd.paths || !upd.texts || !upd.lens)
      abort();
  }
  upd.paths[upd.n] = strdup(path);
  upd.texts[upd.n] = text ? malloc(len + 1) : NULL;
  if (text)
    memcpy(upd.texts[upd.n], text, len);
  upd.lens[upd.n] = text ? len : SIZE_MAX;
  upd.n++;
}

static void upd_clear(void)
{
  for (int i = 0; i < upd.n; i++)
  {
    free(upd.paths[i]);
    free(upd.texts[i]);
  }
  upd.n = 0;
  upd.valid = false;
}

static bool engine_started;

static void apply_update(void)
{
  if (!engine)
    engine = txp_engine_new(cfg.engine, "", cfg.tex_name);
  txp_engine_begin_changes(engine);
  txp_engine_detect_changes(engine);
  for (int i = 0; i < upd.n; i++)
  {
    char key[PATH_MAX], canon[PATH_MAX];
    if (upd.paths[i][0] != '/')
      continue;
    canonical_path(upd.paths[i], canon);
    if (!vfs_key(canon, key))
      continue;
    if (upd.lens[i] == SIZE_MAX)
      interpret_close(key);
    else
      interpret_open(key, upd.texts[i], upd.lens[i]);
  }
  bool changed = txp_engine_end_changes(engine);
  generation = upd.generation;
  upd_clear();
  if (changed || !engine_started)
  {
    engine_started = true;
    current_seq = 0;
    generation_start_ms = now_ms();
    rerun_count = 0;
    crash_retried = false;
    watchdog_killed = false;
    published_pages_this_pass = 0;
    txp_engine_step(engine, true);
    was_running = txp_engine_get_status(engine) == DOC_RUNNING;
  }
  else if (txp_engine_get_status(engine) == DOC_TERMINATED)
  {
    /* Nothing the engine observed changed: the newest publication (or
     * failure) is current for this generation. */
    pbuf b = { 0 };
    pbuf_printf(&b, "{\"event\":\"idle\",\"generation\":%lld,\"seq\":%d}", generation, current_seq);
    emit(&b);
    pbuf_free(&b);
  }
}

static void handle_line(const char *line, size_t len)
{
  json *msg = json_parse(line, len);
  if (!msg)
  {
    emit_simple("error", "protocol", "malformed request");
    return;
  }
  const char *op = json_string(json_get(msg, "op"));
  if (!op)
    emit_simple("error", "protocol", "missing op");
  else if (strcmp(op, "update") == 0)
  {
    double g = 0;
    json_number(json_get(msg, "generation"), &g);
    if ((long long)g > upd.generation || !upd.valid)
      upd.generation = (long long)g;
    json *files = json_get(msg, "files");
    for (int i = 0; files && files->type == JSON_ARRAY && i < files->n; i++)
    {
      const char *path = json_string(json_get(files->items[i], "path"));
      json *text = json_get(files->items[i], "text");
      if (path && text && text->type == JSON_STRING)
        upd_set(path, text->str, text->len);
    }
    json *closed = json_get(msg, "closed");
    for (int i = 0; closed && closed->type == JSON_ARRAY && i < closed->n; i++)
    {
      const char *path = json_string(closed->items[i]);
      if (path)
        upd_set(path, NULL, 0);
    }
    upd.valid = true;
  }
  else if (strcmp(op, "release") == 0)
  {
    double s;
    if (json_number(json_get(msg, "seq"), &s))
      release((int)s);
  }
  else if (strcmp(op, "quit") == 0)
  {
    json_free(msg);
    if (engine)
      txp_engine_free(engine);
    exit(0);
  }
  else
    emit_simple("error", "protocol", "unknown op");
  json_free(msg);
}

/* ------------------------------------------------------------------ */
/* Pass lifecycle                                                      */

static bool log_finished(const tbuf *log)
{
  return contains(log, "Output written on") || contains(log, "No pages of output");
}

static void publish_or_defer(snapshot *s, bool complete)
{
  if (publish(s, complete, generation))
    return;
  /* Backpressure: keep only the newest deferred publication. */
  snapshot_clear(&pending);
  pending = *s;
  memset(s, 0, sizeof *s);
  pending_valid = true;
  pending_complete = complete;
  pending_generation = generation;
}

static void on_pass_end(void)
{
  if (txp_engine_take_barrier(engine))
  {
    fprintf(stderr, "[driver] restarting engine after font barrier\n");
    txp_engine_restart(engine);
    published_pages_this_pass = 0;
    was_running = true;
    return;
  }
  tbuf *log = txp_engine_log(engine);
  bool finished = log_finished(log);
  if (!finished)
  {
    if (watchdog_killed)
    {
      emit_failed(log, "stuck",
                  "TeX made no progress for 20 seconds (possible infinite loop). Edit the document to retry.");
      return;
    }
    if (!crash_retried)
    {
      /* An engine process ended without finishing the document (crash,
       * killed checkpoint). Retry once from a freshly exec'd engine. */
      crash_retried = true;
      fprintf(stderr, "[driver] engine ended unexpectedly (status %d); restarting from scratch\n",
              txp_engine_last_status(engine));
      txp_engine_restart(engine);
      published_pages_this_pass = 0;
      was_running = true;
      return;
    }
    emit_failed(log, "crashed", "The embedded preview engine stopped unexpectedly.");
    return;
  }

  snapshot s = { 0 };
  snapshot_take(&s);
  bool rerun = false;
  if (txp_engine_aux_dirty(engine) && rerun_count < MAX_RERUNS)
  {
    txp_engine_start_finishing(engine);
    rerun = txp_engine_finish_convergence(engine);
    if (rerun)
    {
      rerun_count++;
      was_running = true;
      published_pages_this_pass = 0;
    }
  }
  int pages = xdv_index_page_count(s.index);
  if (pages == 0)
  {
    if (!rerun)
      emit_failed(s.log, "no_pages", "The document produced no pages.");
    snapshot_clear(&s);
    return;
  }
  snapshot_clear(&last_complete);
  last_complete.xdv = tbuf_keep(s.xdv);
  last_complete.synctex = tbuf_keep(s.synctex);
  last_complete.log = tbuf_keep(s.log);
  last_complete.index = xdv_index_new();
  xdv_index_update(last_complete.index, last_complete.xdv->data, last_complete.xdv->len);
  publish_or_defer(&s, !rerun);
  snapshot_clear(&s);
}

static void maybe_publish_intermediate(void)
{
  if (rerun_count > 0)
    return; /* convergence reruns only refine the last pass */
  int pages = xdv_index_page_count(txp_engine_xdv(engine));
  if (pages <= published_pages_this_pass)
    return;
  long long t = now_ms();
  if (t - last_publish_ms < INTERMEDIATE_INTERVAL_MS || t - generation_start_ms < 120)
    return;
  if (unreleased_count() >= MAX_UNRELEASED)
    return;
  snapshot s = { 0 };
  snapshot_take(&s);
  published_pages_this_pass = pages;
  publish(&s, false, generation);
  snapshot_clear(&s);
}

static void run_engine_slice(void)
{
  long long t0 = now_ms();
  while (txp_engine_get_status(engine) == DOC_RUNNING)
  {
    if (!txp_engine_step(engine, false))
      break;
    if (now_ms() - t0 > 25)
      break;
  }
  if (txp_engine_get_status(engine) == DOC_RUNNING)
  {
    if (txp_engine_idle_ms(engine) > WATCHDOG_MS)
    {
      fprintf(stderr, "[driver] watchdog: no engine progress, killing\n");
      watchdog_killed = true;
      txp_engine_kill_running(engine);
    }
    else
      maybe_publish_intermediate();
  }
  if (was_running && txp_engine_get_status(engine) == DOC_TERMINATED)
  {
    was_running = false;
    on_pass_end();
  }
}

/* ------------------------------------------------------------------ */
/* Startup                                                             */

/* Kill our whole process group (the driver and every engine checkpoint),
 * then die from the original signal. Async-signal-safe. */
static void on_fatal_signal(int sig)
{
  signal(SIGTERM, SIG_IGN);
  kill(0, SIGKILL);
  signal(sig, SIG_DFL);
  raise(sig);
}

static void usage_error(const char *msg)
{
  emit_simple("error", "usage", msg);
  exit(2);
}

static void default_engine_path(char out[PATH_MAX])
{
  char self[PATH_MAX] = { 0 };
#ifdef __APPLE__
  uint32_t size = sizeof self;
  if (_NSGetExecutablePath(self, &size) != 0)
    self[0] = 0;
#else
  ssize_t n = readlink("/proc/self/exe", self, sizeof self - 1);
  if (n > 0)
    self[n] = 0;
#endif
  char real[PATH_MAX];
  if (self[0] && realpath(self, real))
  {
    char *slash = strrchr(real, '/');
    if (slash)
      *slash = 0;
    snprintf(out, PATH_MAX, "%s/pitex-preview-xetex", real);
  }
  else
    snprintf(out, PATH_MAX, "pitex-preview-xetex");
}

int main(int argc, char **argv)
{
  signal(SIGPIPE, SIG_IGN);
  /* As process-group leader (Pitex starts the helper that way), take every
   * engine checkpoint down with us on termination or a crash. */
  if (getpgrp() == getpid())
  {
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = on_fatal_signal;
    sa.sa_flags = SA_RESETHAND;
    const int fatal[] = { SIGTERM, SIGHUP, SIGINT, SIGABRT, SIGSEGV, SIGBUS, SIGILL, SIGFPE };
    for (size_t i = 0; i < sizeof fatal / sizeof fatal[0]; i++)
      sigaction(fatal[i], &sa, NULL);
  }
#ifdef __linux__
  prctl(PR_SET_PDEATHSIG, SIGTERM);
#endif
  const char *root = NULL, *main_rel = NULL, *out = NULL, *eng = NULL, *cache = NULL;
  for (int i = 1; i < argc; i++)
  {
    if (!strcmp(argv[i], "--root") && i + 1 < argc)
      root = argv[++i];
    else if (!strcmp(argv[i], "--main") && i + 1 < argc)
      main_rel = argv[++i];
    else if (!strcmp(argv[i], "--out") && i + 1 < argc)
      out = argv[++i];
    else if (!strcmp(argv[i], "--engine") && i + 1 < argc)
      eng = argv[++i];
    else if (!strcmp(argv[i], "--cache") && i + 1 < argc)
      cache = argv[++i];
    else if (!strcmp(argv[i], "--version"))
    {
      printf("pitex-preview 1 (TeXpresso e8df7709077b-derived engine)\n");
      return 0;
    }
    else
      usage_error("usage: pitex-preview --root DIR --main REL --out DIR [--engine PATH] [--cache DIR]");
  }
  if (!root || !main_rel || !out)
    usage_error("--root, --main and --out are required");
  if (!realpath(root, cfg.root))
    usage_error("project root does not exist");
  if (!realpath(out, cfg.out))
    usage_error("output directory does not exist");
  if (main_rel[0] == '/' || strstr(main_rel, ".."))
    usage_error("--main must be relative to --root");
  char main_abs[PATH_MAX];
  snprintf(main_abs, sizeof main_abs, "%s/%s", cfg.root, main_rel);
  char main_real[PATH_MAX];
  if (!realpath(main_abs, main_real))
    usage_error("main document does not exist");
  snprintf(cfg.main_dir, sizeof cfg.main_dir, "%s", main_real);
  char *slash = strrchr(cfg.main_dir, '/');
  *slash = 0;
  snprintf(cfg.tex_name, sizeof cfg.tex_name, "%s", slash + 1);
  snprintf(cfg.job, sizeof cfg.job, "%s", cfg.tex_name);
  char *dot = strrchr(cfg.job, '.');
  if (dot)
    *dot = 0;
  if (eng)
    snprintf(cfg.engine, sizeof cfg.engine, "%s", eng);
  else
    default_engine_path(cfg.engine);
  if (access(cfg.engine, X_OK) != 0)
  {
    emit_simple("error", "no_engine", "pitex-preview-xetex helper not found");
    return 2;
  }
  if (cache)
    setenv("PITEX_PREVIEW_CACHE", cache, 1);

  /* Driver and engine diagnostics go to a capped log file in the session
   * directory, never into the client's pipes. */
  char logpath[PATH_MAX];
  snprintf(logpath, sizeof logpath, "%s/driver.log", cfg.out);
  log_fd = open(logpath, O_WRONLY | O_CREAT | O_APPEND | O_TRUNC, 0600);
  if (log_fd >= 0)
    dup2(log_fd, STDERR_FILENO);

  if (chdir(cfg.main_dir) != 0)
    usage_error("cannot enter the main document directory");

  /* TeX Live/MacTeX (kpsewhich) is the only file provider — the Tectonic
   * network bundle path was removed; the engine always runs -texlive. */
  if (!texlive_available())
  {
    emit_simple("error", "no_tex", "No TeX Live installation found (kpsewhich must be on PATH).");
    return 2;
  }

  writer = xdv2pdf_new((xdv_resolver){ NULL, resolve });
  emit_simple("ready", NULL, NULL);

  pbuf in = { 0 };
  for (;;)
  {
    struct pollfd fds[2];
    int nfds = 1;
    fds[0].fd = STDIN_FILENO;
    fds[0].events = POLLIN;
    int efd = engine && txp_engine_get_status(engine) == DOC_RUNNING ? txp_engine_fd(engine) : -1;
    if (efd >= 0)
    {
      fds[1].fd = efd;
      fds[1].events = POLLIN;
      nfds = 2;
    }
    int timeout = efd >= 0 ? 50 : -1;
    int r = poll(fds, nfds, upd.valid ? 0 : timeout);
    if (r < 0 && errno != EINTR)
      break;
    if (r > 0 && (fds[0].revents & (POLLIN | POLLHUP | POLLERR)))
    {
      char buf[65536];
      ssize_t n = read(STDIN_FILENO, buf, sizeof buf);
      if (n == 0 || (n < 0 && errno != EINTR && errno != EAGAIN))
        break; /* client closed the session */
      if (n > 0)
      {
        pbuf_append(&in, buf, (size_t)n);
        size_t start = 0;
        for (size_t i = 0; i < in.len; i++)
          if (in.data[i] == '\n')
          {
            if (i > start)
              handle_line((char *)in.data + start, i - start);
            start = i + 1;
          }
        memmove(in.data, in.data + start, in.len - start);
        in.len -= start;
        /* Drain everything already queued before applying: coalesce
         * bursts of updates into one rollback. */
        struct pollfd p = { STDIN_FILENO, POLLIN, 0 };
        if (poll(&p, 1, 0) > 0 && (p.revents & POLLIN))
          continue;
      }
    }
    if (upd.valid)
      apply_update();
    if (engine && (was_running || txp_engine_get_status(engine) == DOC_RUNNING))
    {
      was_running = was_running || txp_engine_get_status(engine) == DOC_RUNNING;
      run_engine_slice();
    }
  }
  if (engine)
    txp_engine_free(engine);
  return 0;
}
