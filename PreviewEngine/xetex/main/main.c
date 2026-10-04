#ifdef __APPLE__
# include <sys/syslimits.h>
#else
# include <linux/limits.h>
#endif

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/file.h>
#include <time.h>
#include <unistd.h>
#include "formats.h"
#include "tectonic_bridge_core.h"
#include "tectonic_bridge_core_generated.h"
#include "providers.h"
#include "texpresso_protocol.h"
#include "common.h"
#include "../engine/xetex-xetexd.h"
#define LOG 0

/**
 * Global definitions
 */

// Name of root document
const char *primary_document = NULL;

// Path of the last opened file
char last_open[PATH_MAX + 1];


// Connect to TeXpresso
bool use_texpresso = 0;
txp_client *texpresso = NULL;

// Generate a new format file
bool regenerate_format = 0;

// A file in which dependencies are recorded (or NULL if not necessary)
FILE *dependency_tape;

// Name of the format to use
const char *format_name;

static const char *format_path(const char *ext);

/* Two engines with an empty cache can race to write the same format:
 * serialize validate+bootstrap on a lock file and publish .fmt/.deps
 * atomically (tmp + rename) so no engine ever reads a partial dump. */
static int format_lock_fd = -1;

static void lock_format(void);
static void unlock_format(void);

static void lock_format(void)
{
  const char *fmt = format_path(".fmt");
  if (!fmt)
    return;
  char lock[PATH_MAX + 1];
  snprintf(lock, sizeof lock, "%s.lock", fmt);
  format_lock_fd = open(lock, O_CREAT | O_RDWR, 0600);
  if (format_lock_fd < 0)
    return; /* best effort: an unwritable lock still allows bootstrap */
  if (flock(format_lock_fd, LOCK_EX) != 0)
  {
    close(format_lock_fd);
    format_lock_fd = -1;
  }
}

static void unlock_format(void)
{
  if (format_lock_fd >= 0)
  {
    flock(format_lock_fd, LOCK_UN);
    close(format_lock_fd);
    format_lock_fd = -1;
  }
}

/* The in-flight .fmt handle: written to .fmt.tmp, renamed on close. */
static FILE *pending_fmt;
static char pending_fmt_path[PATH_MAX + 1];

// Buffer for printing format string
#define FORMAT_BUF_SIZE 4096
char format_buf[FORMAT_BUF_SIZE];

NORETURN PRINTF_FUNC(1, 2) int _tt_abort(const char *format, ...)
{
  fprintf(stderr, "Fatal error, aborting: ");
  va_list ap;
  va_start(ap, format);
  vfprintf(stderr, format, ap);
  va_end(ap);
  fprintf(stderr, "\n");
  do_abort();
}

int ttstub_shell_escape(const unsigned short *cmd, size_t len)
{
  // FIXME: input is UTF-16
  log_proc(logging, "%.*s, %d", (int)len, (char *)cmd, (int)len);
  return 0;
}

// Input management
static FILE *input_as_file(ttbc_input_handle_t *h)
{
  if (texpresso)
    abort();
  return (void *)h;
}
static ttbc_input_handle_t *file_as_input(FILE *h)
{
  if (texpresso)
    abort();
  return (void *)h;
}

// Allocation of IDs for texpresso protocol
uint64_t allocated[16];

static txp_file_id next_id(void)
{
  for (int i = 0; i < 16; i++)
    if (allocated[i] != UINT64_MAX)
      for (int j = 0; j < 64; ++j)
        if ((allocated[i] & ((uint64_t)1 << j)) == 0)
          return (i << 6) | j;
  fprintf(stderr, "All ids have been allocated\n");
  abort();
}

static void alloc_id(txp_file_id id)
{
  int cell = id >> 6;
  if (cell < 0 || cell >= 16)
    abort();
  int offset = id & 63;
  if ((allocated[cell] & ((uint64_t)1 << offset)) != 0)
    abort();
  allocated[cell] |= (uint64_t)1 << offset;
}

static void release_id(txp_file_id id)
{
  int cell = id >> 6;
  if (cell < 0 || cell >= 16)
    abort();
  int offset = id & 63;
  if ((allocated[cell] & (1 << offset)) == 0)
    abort();
  allocated[cell] &= ~((uint64_t)1 << offset);
}

typedef struct
{
  txp_file_id id;
  union
  {
    FILE *file;
    struct
    {
      int file_size;
      int file_pos;
      int generation;
      int buf_pos, buf_len;
      uint8_t buffer[1024];
    };
  };
} txp_input;

typedef struct {
  txp_file_id id;
  FILE *file;
} txp_input_file;

static txp_input *input_as_txp(ttbc_input_handle_t *h)
{
  if (!texpresso) abort();
  return (void *)h;
}

static ttbc_input_handle_t *txp_as_input(txp_input *h)
{
  if (!texpresso) abort();
  return (void *)h;
}

static enum txp_file_kind
kind_of_ttbc_format(ttbc_file_format format)
{
#define CASE(x) case TTBC_FILE_FORMAT_##x: return TXP_KIND_##x
  switch (format)
  {
    CASE(AFM);
    CASE(BIB);
    CASE(BST);
    CASE(CMAP);
    CASE(CNF);
    CASE(ENC);
    CASE(FORMAT);
    CASE(FONT_MAP);
    CASE(MISC_FONTS);
    CASE(OFM);
    CASE(OPEN_TYPE);
    CASE(OVF);
    CASE(PICT);
    CASE(PK);
    CASE(PROGRAM_DATA);
    CASE(SFD);
    CASE(TEX);
    CASE(TEX_PS_HEADER);
    CASE(TFM);
    CASE(TRUE_TYPE);
    CASE(TYPE1);
    CASE(VF);
    case TTBC_FILE_FORMAT_TECTONIC_PRIMARY:
      return TXP_KIND_PRIMARY;
    default:
      abort();
  }
#undef CASE
}

ttbc_input_handle_t *ttstub_input_open(const char *path,
                                       ttbc_file_format format,
                                       int is_gz)
{
  log_proc(logging, "path:%s, format:%s, is_gz:%d", path,
           ttbc_file_format_to_string(format), is_gz);

  if (is_gz != 0)
    do_abortf("GZ compression not supported");

  if (texpresso && format == TTBC_FILE_FORMAT_FORMAT)
  {
    const char *cached = format_path(".fmt");
    if (!cached)
      return NULL;

    FILE *f = fopen(cached, "rb");
    if (!f)
      return NULL;

    strcpy(last_open, cached);

    txp_input_file *input = calloc(1, sizeof(txp_input_file));
    if (!input)
      abort();
    input->id = -1;
    input->file = f;

    return txp_as_input((txp_input *)input);
  }

  if (texpresso)
  {
    txp_file_id id = next_id();

    char *ipath =
      txp_open(texpresso, id, path, kind_of_ttbc_format(format), TXP_READ);

    if (ipath)
    {
      strcpy(last_open, ipath);
      free(ipath);

      txp_input *input = calloc(1, sizeof(txp_input));
      if (!input)
        abort();

      alloc_id(id);

      input->id = id;
      input->file_size = -1;
      input->generation = txp_generation(texpresso);
      return txp_as_input(input);
    }
  }

  FILE *f = fopen(path, "rb");
  const void *buffer = NULL;
  size_t len = 0;
  char tmp[PATH_MAX + 1];

  if (!f && format == TTBC_FILE_FORMAT_FORMAT)
  {
    const char *cached = format_path(".fmt");
    if (cached)
      f = fopen(cached, "rb");
  }

  if (f)
    strcpy(last_open, path);

  if (!f)
  {
    for (const char **exts = format_extensions(format); !f && *exts; exts++)
    {
      strcpy(tmp, path);
      strcat(tmp, *exts);
      f = fopen(tmp, "rb");
      if (f)
        strcpy(last_open, tmp);
    }
  }

  if (!f)
  {
    if (LOG)
      fprintf(stderr, "input_open: failed to open file %s\n", path);

    const char *tlpath = texlive_file_path(path, dependency_tape);
    for (const char **exts = format_extensions(format); !tlpath && *exts;
         exts++)
    {
      strcpy(tmp, path);
      strcat(tmp, *exts);
      tlpath = texlive_file_path(tmp, NULL);
    }
    if (tlpath)
    {
      strcpy(last_open, tlpath);
      f = fopen(tlpath, "rb");
    }
    else if (LOG)
      fprintf(stderr, "Texlive failed to provide the file.\n");
  }

  if (texpresso)
  {
    if (!f)
      return NULL;

    txp_input_file *input = calloc(1, sizeof(txp_input_file));
    if (!input)
      abort();
    input->id = -1;
    input->file = f;

    return txp_as_input((txp_input *)input);
  }
  else
    return file_as_input(f);
}

ttbc_input_handle_t *ttstub_input_open_primary(void)
{
  log_proc(logging, "");
  if (!primary_document)
  {
    do_abortf("Document name as not been specified");
  }
  return ttstub_input_open(primary_document, TTBC_FILE_FORMAT_TEX, 0);
}

int ttstub_input_close(ttbc_input_handle_t *handle)
{
  if (texpresso)
  {
    txp_input *input = input_as_txp(handle);
    if (input->id == -1)
    {
      int result = fclose(input->file);
      free(input);
      return result;
    }

    txp_close(texpresso, input->id);
    release_id(input->id);
    free(input);
    return 0;
  }

  return fclose(input_as_file(handle));
}

int ttstub_input_getc(ttbc_input_handle_t *handle)
{
  if (texpresso)
  {
    txp_input *input = input_as_txp(handle);
    if (input->id == -1)
      return getc(input->file);

    if (input->generation != txp_generation(texpresso))
    {
      input->generation = txp_generation(texpresso);
      input->file_pos += input->buf_pos;
      input->buf_pos = input->buf_len = 0;
    }

    if (input->buf_pos >= input->buf_len)
    {
      input->file_pos += input->buf_len;
      input->buf_pos = 0;
      input->buf_len = txp_read(texpresso, input->id, input->file_pos,
                                input->buffer, sizeof(input->buffer));
      if (input->buf_len == 0)
        return EOF;
    }

    int result = input->buffer[input->buf_pos++];
    txp_seen(texpresso, input->id, input->file_pos + input->buf_pos);
    return result;
  }

  return getc(input_as_file(handle));
}

time_t ttstub_input_get_mtime(ttbc_input_handle_t *handle)
{
  struct stat file_stat;
  if (texpresso)
  {
    txp_input *input = input_as_txp(handle);
    if (input->id == -1)
    {
      if (fstat(fileno(input->file), &file_stat) == -1)
        return 0;  // Error getting file statistics
      return file_stat.st_mtime;
    }

    return txp_mtime(texpresso, input->id);
  }

  if (fstat(fileno(input_as_file(handle)), &file_stat) == -1)
    return 0;  // Error getting file statistics
  return file_stat.st_mtime;
}

uint32_t txp_input_size(txp_input *input)
{
  if (input->file_size == -1)
    input->file_size = txp_size(texpresso, input->id);
  return input->file_size;
}

size_t ttstub_input_get_size(ttbc_input_handle_t *handle)
{
  struct stat file_stat;
  if (texpresso)
  {
    txp_input *input = input_as_txp(handle);
    if (input->id == -1)
    {
      if (fstat(fileno(input->file), &file_stat) == -1)
        return 0;  // Error getting file statistics
      return file_stat.st_size;
    }

    return txp_input_size(input);
  }
  if (fstat(fileno(input_as_file(handle)), &file_stat) == -1)
    return 0;  // Error getting file statistics
  return file_stat.st_size;
}

size_t ttstub_input_seek(ttbc_input_handle_t *handle,
                         ssize_t offset,
                         int whence)
{
  if (texpresso)
  {
    txp_input *input = input_as_txp(handle);
    if (input->id == -1)
      return fseek(input->file, offset, whence);

    int ofs = offset;
    if (whence == SEEK_CUR)
      ofs += input->file_pos + input->buf_pos;
    else if (whence == SEEK_END)
      ofs += txp_input_size(input);

    if (ofs < 0)
      return -1;

    int buf_pos = ofs - input->file_pos;
    if (buf_pos > 0 && buf_pos <= input->buf_len)
      input->buf_pos = buf_pos;
    else
    {
      input->file_pos = ofs;
      input->buf_pos = input->buf_len = 0;
    }

    return input->file_pos + input->buf_pos;
  }
  return fseek(input_as_file(handle), offset, whence);
}

static ssize_t internal_input_read(ttbc_input_handle_t *handle, char *data, size_t len)
{
  if (texpresso)
  {
    txp_input *input = input_as_txp(handle);
    if (input->id == -1)
      return fread(data, 1, len, input->file);

    if (input->generation != txp_generation(texpresso))
    {
      input->generation = txp_generation(texpresso);
      input->file_pos += input->buf_pos;
      input->buf_pos = input->buf_len = 0;
    }

    if (input->buf_pos < input->buf_len)
    {
      if (len >= input->buf_len - input->buf_pos)
        len = input->buf_len - input->buf_pos;
      memmove(data, input->buffer + input->buf_pos, len);
      input->buf_pos += len;
      txp_seen(texpresso, input->id, input->file_pos + input->buf_pos);
      return len;
    }

    if (len < sizeof(input->buffer))
    {
      input->file_pos = input->file_pos + input->buf_len;
      input->buf_len = txp_read(texpresso, input->id, input->file_pos, input->buffer, len);
      if (len > input->buf_len)
        len = input->buf_len;
      memmove(data, input->buffer, len);
      input->buf_pos = len;
      txp_seen(texpresso, input->id, input->file_pos + input->buf_pos);
      return len;
    }
    else
    {
      input->file_pos = input->file_pos + input->buf_len;
      len = txp_read(texpresso, input->id, input->file_pos, data, len);
      input->file_pos += len;
      input->buf_pos = input->buf_len = 0;
      txp_seen(texpresso, input->id, input->file_pos + input->buf_pos);
      return len;
    }
  }

  return fread(data, 1, len, input_as_file(handle));
}

ssize_t ttstub_input_read(ttbc_input_handle_t *handle, char *data, size_t len)
{
  ssize_t result = internal_input_read(handle, data, len);

  if (result <= 0)
      return result;

  while (result <= len)
  {
    ssize_t delta = internal_input_read(handle, data + result, len - result);
    if (delta <= 0)
      return result;
    result += delta;
  }

  return result;
}

int ttstub_input_ungetc(ttbc_input_handle_t *handle, int ch)
{
  if (texpresso)
  {
    txp_input *input = input_as_txp(handle);
    if (input->id == -1)
      return ungetc(ch, input->file);

    if (input->buf_pos == 0)
      return EOF;

    input->buffer[--input->buf_pos] = ch;
    return ch;
  }

  return ungetc(ch, input_as_file(handle));
}

ssize_t ttstub_get_last_input_abspath(char *buffer, size_t len)
{
  size_t llen = strlen(last_open);
  memmove(buffer, last_open, llen > len ? len : llen);
  if (llen < len)
    buffer[llen] = '\0';
  return llen;
}

// Output management
static FILE *output_as_file(ttbc_output_handle_t *h)
{
  if (texpresso)
    abort();
  return (void *)h;
}

static ttbc_output_handle_t *file_as_output(FILE *h)
{
  if (texpresso)
    abort();
  return (void *)h;
}

static txp_file_id output_as_txp(ttbc_output_handle_t *p)
{
  uintptr_t h = (uintptr_t)p;
  if (!texpresso || (h > 1024 && h != (uintptr_t)-1)) abort();
  return h;
}

static ttbc_output_handle_t *txp_as_output(txp_file_id h)
{
  uintptr_t p = h;
  if (!texpresso) abort();
  return (void *)p;
}


int ttstub_output_flush(ttbc_output_handle_t *handle)
{
  if (texpresso)
  {
    txp_flush(texpresso);
    return 0;
  }
  else
    return fflush(output_as_file(handle));
}

int ttstub_output_close(ttbc_output_handle_t *handle)
{
  if (texpresso)
  {
    txp_close(texpresso, output_as_txp(handle));
    return 0;
  }
  FILE *f = output_as_file(handle);
  if (pending_fmt && f == pending_fmt)
  {
    /* The format is published atomically: rename only on clean close. */
    pending_fmt = NULL;
    if (fclose(f) != 0)
    {
      unlink(pending_fmt_path);
      return EOF;
    }
    char final[PATH_MAX + 1];
    size_t n = strlen(pending_fmt_path);
    if (n <= 4 || n - 4 >= sizeof final)
    {
      unlink(pending_fmt_path);
      return EOF;
    }
    memcpy(final, pending_fmt_path, n - 4);
    final[n - 4] = 0;
    if (rename(pending_fmt_path, final) != 0)
    {
      unlink(pending_fmt_path);
      return EOF;
    }
    return 0;
  }
  return fclose(f);
}

ttbc_output_handle_t *ttstub_output_open(char const *path, int is_gz)
{
  log_proc(logging, "path:%s, is_gz:%d", path, is_gz);
  if (is_gz)
    do_abortf("is_gz not supported");

  if (texpresso)
  {
    txp_file_id id = next_id();
    char *opath = txp_open(texpresso, id, path, TXP_KIND_OTHER, TXP_WRITE);
    if (!opath)
      return NULL;
    free(opath);
    alloc_id(id);
    return txp_as_output(id);
  }

  if (in_initex_mode && path)
  {
    const char *p = path;
    while (*p && *p != '/') p++;
    if (!*p)
      path = format_path(path);
  }

  return file_as_output(fopen(path, "wb"));
}

ttbc_output_handle_t *ttstub_output_open_format(char const *path, int is_gz)
{
  log_proc(logging, "path:%s, is_gz:%d", path, is_gz);
  if (texpresso || !in_initex_mode)
    abort();
  path = format_path(".fmt");
  if (!path)
    return NULL;
  /* Write texlive-xelatex.ini.fmt.tmp; renamed to .fmt on clean close. */
  snprintf(pending_fmt_path, sizeof pending_fmt_path, "%s.tmp", path);
  pending_fmt = fopen(pending_fmt_path, "wb");
  return file_as_output(pending_fmt);
}

ttbc_output_handle_t *ttstub_output_open_stdout(void)
{
  if (texpresso)
    return (void*)(uintptr_t)(-1);

  return file_as_output(stdout);
}

int ttstub_output_putc(ttbc_output_handle_t *handle, int c)
{
  if (texpresso)
  {
    txp_putc(texpresso, output_as_txp(handle), c);
    return c;
  }
  else
    return putc(c, output_as_file(handle));
}

size_t ttstub_output_write(ttbc_output_handle_t *handle,
                           const char *data,
                           size_t len)
{
  if (texpresso)
  {
    txp_append(texpresso, output_as_txp(handle), data, len);
    return len;
  }
  else
    return fwrite(data, 1, len, output_as_file(handle));
}

PRINTF_FUNC(2, 3)
int ttstub_fprintf(ttbc_output_handle_t *handle, const char *format, ...)
{
  va_list ap;
  va_start(ap, format);
  int len = vsnprintf(format_buf, FORMAT_BUF_SIZE, format, ap);
  va_end(ap);
  return ttstub_output_write(handle, format_buf, len);
}

// Error diagnostics

enum
{
  DIAG_NONE,
  DIAG_WARN,
  DIAG_ERROR,
} diag_kind;

char diag_buf[1024];
size_t diag_len = 0;
intptr_t curr_diag = 0;

static void diag_write(ttbc_diagnostic_t *diag, const char *buf, ssize_t len)
{
  if ((intptr_t)diag != curr_diag)
    do_abortf("diagnostic: use after free");
  if (diag_len + len > sizeof(diag_buf))
    len = sizeof(diag_buf) - diag_len;
  memmove(diag_buf + diag_len, buf, len);
  diag_len += len;
}

void ttbc_diag_append(ttbc_diagnostic_t *diag, const char *text)
{
  diag_write(diag, text, strlen(text));
}

ttbc_diagnostic_t *ttbc_diag_begin_error(void)
{
  if (diag_kind != DIAG_NONE)
    do_abortf("diagnostic: unexpected nested diagnostics");
  diag_kind = DIAG_ERROR;
  diag_len = 0;
  return (void *)(++curr_diag);
}

ttbc_diagnostic_t *ttbc_diag_begin_warning(void)
{
  if (diag_kind != DIAG_NONE)
    do_abortf("Unexpected nested diagnostics");
  diag_kind = DIAG_WARN;
  diag_len = 0;
  return (void *)(++curr_diag);
}

void ttstub_diag_finish(ttbc_diagnostic_t *diag)
{
  if ((intptr_t)diag != curr_diag)
    do_abortf("diagnostic: use after free");
  switch (diag_kind)
  {
    case DIAG_WARN:
      fprintf(stderr, "Warning: ");
      break;

    case DIAG_ERROR:
      fprintf(stderr, "Error: ");
      break;

    default:
      do_abortf("diagnostic: double free");
  }
  diag_kind = DIAG_NONE;

  fprintf(stderr, "%.*s\n", (int)diag_len, diag_buf);
}

static void diag_vprintf(ttbc_diagnostic_t *diag,
                         const char *format,
                         va_list ap)
{
  diag_write(diag, format_buf,
             vsnprintf(format_buf, FORMAT_BUF_SIZE, format, ap));
}

PRINTF_FUNC(2, 3)
void ttstub_diag_printf(ttbc_diagnostic_t *diag, const char *format, ...)
{
  log_proc(logging, "\"%s\", ...", format);
  va_list ap;
  va_start(ap, format);
  diag_vprintf(diag, format, ap);
  va_end(ap);
}

// Error helpers
PRINTF_FUNC(1, 2) void ttstub_issue_warning(const char *format, ...)
{
  ttbc_diagnostic_t *diag = ttbc_diag_begin_warning();
  va_list ap;
  va_start(ap, format);
  diag_vprintf(diag, format, ap);
  va_end(ap);
  ttstub_diag_finish(diag);
}

PRINTF_FUNC(1, 2) void ttstub_issue_error(const char *format, ...)
{
  ttbc_diagnostic_t *diag = ttbc_diag_begin_error();
  va_list ap;
  va_start(ap, format);
  diag_vprintf(diag, format, ap);
  va_end(ap);
  ttstub_diag_finish(diag);
}

// FIXME: Implement bounds caching later

int ttstub_pic_get_cached_bounds(const char *name, int type, int page, float bounds[4])
{
  if (texpresso)
    return txp_gpic(texpresso, name, type, page, bounds);

  return 0;
}

void ttstub_pic_set_cached_bounds(const char *name, int type, int page, const float bounds[4])
{
  if (texpresso)
    txp_spic(texpresso, name, type, page, bounds);
}

// Pitex: macOS cannot safely load platform (CoreText) fonts in a process
// created by fork() without exec. A checkpoint child reaching such a load
// reports a font barrier and exits; the driver restarts from a freshly
// exec'd engine and delays checkpoints past this point. Other platforms
// (fontconfig) do not need this; PITEX_PREVIEW_FONT_BARRIER=1 enables the
// same path there so the driver's recovery can be exercised on Linux.
extern bool txp_forked_child;

void pitex_guard_platform_font(void)
{
#ifdef XETEX_MAC
  const bool active = true;
#else
  const char *env = getenv("PITEX_PREVIEW_FONT_BARRIER");
  const bool active = env && env[0] == '1';
#endif
  if (active && texpresso && txp_forked_child)
  {
    fprintf(stderr, "[pitex] platform font requested in checkpoint child\n");
    txp_font_barrier(texpresso);
    _exit(0);
  }
}

// Entry point
static void usage(char *argv0)
{
  fprintf(stderr,
      "Usage: %s [-texpresso] [-regenerate-format] "
      "<path.tex>\n"
      "Run XeTeX engine on <path.tex> using packages from TeX Live.\n"
      "\n"
      "Options:\n"
      "  -texpresso   Internal (route I/O through TeXpresso)\n"
      "  -regenerate-format  Force generation of a fresh format file\n",
      argv0);
}

// Generate everything based on 1738978143
// (February 8, 2025)
// #define EXECUTION_DATE 1738978143


static const char *format_path(const char *ext)
{
  /* TeX Live is the only distribution provider. */
  if (!ext || (*ext == '.' || *ext == '-'))
    return cache_path("format", "texlive-", format_name, ext);
  else
    return cache_path("format", "texlive-", format_name, "-", ext);
}

static bool validate_format(void)
{
  if (regenerate_format)
    return 0;

  const char *path;

  path = format_path(".fmt");
  if (!path || access(path, R_OK) != 0)
    return 0;

  path = format_path(".deps");
  if (!path)
    return 0;

  FILE *tape = fopen(path, "rb");
  if (!tape)
    return 0;

  bool result = texlive_check_dependencies(tape);

  fclose(tape);

  return result;
}

static bool bootstrap_format(void)
{
  /* .deps.tmp → .deps on success: a second engine never sees a torn tape */
  const char *path = format_path(".deps");
  if (!path)
    return 0;
  char deps_tmp[PATH_MAX + 1];
  snprintf(deps_tmp, sizeof deps_tmp, "%s.tmp", path);

  dependency_tape = fopen(deps_tmp, "wb");
  if (!dependency_tape)
    return 0;

  in_initex_mode = true;
  primary_document = format_name;
  tt_history_t result =
      tt_run_engine("texpresso.fmt", format_name, 0);
  in_initex_mode = false;
  primary_document = NULL;

  if (dependency_tape)
  {
    fclose(dependency_tape);
    dependency_tape = NULL;
  }

  fprintf(stderr, "Format generation: ");
  switch (result)
  {
    case HISTORY_SPOTLESS:
      fprintf(stderr, "Spotless execution.\n");
      break;
    case HISTORY_WARNING_ISSUED:
      fprintf(stderr, "Warnings issued.\n");
      break;
    case HISTORY_ERROR_ISSUED:
      fprintf(stderr, "Errors issued.\n");
      break;
    case HISTORY_FATAL_ERROR:
      fprintf(stderr, "Aborted with a fatal error.\n");
      break;
  }

  if (result != HISTORY_SPOTLESS)
  {
    /* Ditch the torn tape/fmt; a valid pair in place stays untouched. */
    unlink(format_path(".fmt"));
    unlink(format_path(".deps"));
    unlink(deps_tmp);
    char fmt_tmp[PATH_MAX + 1];
    snprintf(fmt_tmp, sizeof fmt_tmp, "%s.tmp", format_path(".fmt"));
    unlink(fmt_tmp);
  }
  else
  {
    if (rename(deps_tmp, format_path(".deps")) != 0)
    {
      unlink(format_path(".fmt"));
      unlink(format_path(".deps"));
      result = HISTORY_FATAL_ERROR;
    }
  }

  return (result == HISTORY_SPOTLESS);
}

static const char *format_path(const char *ext);

int main(int argc, char **argv)
{
  const char *doc_path = NULL;
  bool dashdash = 0;

  for (int i = 1; i < argc; i++)
  {
    if (!dashdash && argv[i][0] == '-')
    {
      if (strcmp(argv[i], "-texpresso") == 0)
        use_texpresso = 1;
      else if (strcmp(argv[i], "-regenerate-format") == 0)
        regenerate_format = 1;
      else if (strcmp(argv[i], "--") == 0)
        dashdash = 1;
      else
      {
        fprintf(stderr, "Unknown option: %s\n", argv[i]);
        usage(argv[0]);
        return 1;
      }
    }
    else if (doc_path == NULL)
      doc_path = argv[i];
    else
    {
      fprintf(stderr,
              "Extraneous argument: %s\n"
              "Only one input file can be provided\n",
              argv[i]);
      usage(argv[0]);
      return 1;
    }
  }

  if (!texlive_available())
  {
    fprintf(stderr,
            "Cannot find TeX Live (ensure the kpsewhich command is "
            "executable).\n"
            "A TeX Live or MacTeX installation is required.\n");
    usage(argv[0]);
    return 1;
  }

  fprintf(stderr, "Using TeX Live.\n");

  // Generate format if necessary
  format_name = "xelatex.ini";
  /* A second engine on an empty cache waits here instead of racing the
   * format write; held across validate+bootstrap so both see one pair. */
  lock_format();
  if (!validate_format() && !bootstrap_format())
  {
    fprintf(stderr, "Failed to generate format.\n");
    unlock_format();
    return 1;
  }
  unlock_format();

  // Initialize TeXpresso if necessary
  if (use_texpresso)
  {
    const char *texpresso_fd = getenv("TEXPRESSO_FD");
    int fd = 0;
    if (texpresso_fd)
      while (*texpresso_fd >= '0' && *texpresso_fd <= '9')
        fd = fd * 10 + *texpresso_fd++ - '0';
    if (!fd || !texpresso_fd || *texpresso_fd)
    {
      fprintf(stderr, "Flag -texpresso is for internal use only.\n");
      return 1;
    }
    texpresso = txp_connect(fdopen(fd, "r+"));
    if (!texpresso)
    {
      fprintf(stderr, "Failed to connect to TeXpresso.\n");
      return 1;
    }
    synctex_enabled = 1;
    /* Pitex: keep standard SyncTeX output (no TeXpresso "/<tag>" records);
     * the driver writes it next to the preview PDF for the synctex CLI. */
    synctex_texpresso_extension = 0;
  }

  // Generate document
  if (!doc_path)
  {
    fprintf(stderr, "Expecting a .tex input file.\n");
    usage(argv[0]);
    return 1;
  }

  in_initex_mode = false;
  primary_document = doc_path;

  // Determine build date.
  // Use SOURCE_DATE_EPOCH if it set.
  // Default to current timestamp otherwise.
  time_t build_date;

  char *epoch_env = getenv("SOURCE_DATE_EPOCH");

  if (epoch_env != NULL)
    build_date = (time_t)strtoll(epoch_env, NULL, 10);
  else
    build_date = time(NULL);

  // Run engine.
  tt_history_t result =
      tt_run_engine("texpresso.fmt", primary_document, build_date);

  fprintf(stderr, "Document generation: ");
  switch (result)
  {
    case HISTORY_SPOTLESS:
      fprintf(stderr, "Spotless execution.\n");
      break;
    case HISTORY_WARNING_ISSUED:
      fprintf(stderr, "Warnings issued.\n");
      break;
    case HISTORY_ERROR_ISSUED:
      fprintf(stderr, "Errors issued.\n");
      break;
    case HISTORY_FATAL_ERROR:
      fprintf(stderr, "Aborted with a fatal error.\n");
      break;
  }
  if (result == HISTORY_SPOTLESS)
    return 0;
  return 1;
}
