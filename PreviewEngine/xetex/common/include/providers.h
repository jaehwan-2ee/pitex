#ifndef PROVIDERS_H_
#define PROVIDERS_H_

#include <stdio.h>
#include <stdbool.h>

/* The Tectonic provider (bundles fetched over the network via popen/
 * system) was removed for the Pitex editing preview: TeX Live/MacTeX is
 * the documented requirement and the app never selects it. */

/***********/
/* TEXLIVE */
/***********/

/**
 * Check if TeX Live is available on the system.
 *
 * @return true if TeX Live is available, false otherwise.
 */
bool texlive_available(void);

/**
 * Retrieve the file path for a given TeX Live file.
 *
 * @param name The name of the TeX Live file to locate.
 * @param record_dependency A writable file stream to record the dependency, or NULL.
 * @return The file path of the TeX Live file, or NULL if not found.
 */
const char *texlive_file_path(const char *name, FILE *record_dependency);

/**
 * Check if all recorded dependencies for TeX Live are still up to date.
 *
 * @param record A file stream to read the dependencies.
 * @return true if dependencies are still up, false otherwise.
 */
bool texlive_check_dependencies(FILE *record);

/********************/
/* CACHE MANAGEMENT */
/********************/

/**
 * Construct a cache path based on the provided folder and name.
 *
 * This function constructs a cache path by combining the folder and name with
 * a base path derived from the XDG_CACHE_HOME or HOME environment variables.
 * It ensures that the directory structure exists and is properly normalized.
 *
 * @param folder The subdirectory name within the cache path.
 * @param name The file name within the cache path.
 * @return The constructed cache path, or NULL if an error occurs.
 *         The returned buffer is managed by the function and valid until the
 *         next call.
 */
const char *cache_path_(const char *folder, const char *name[]);
#define cache_path(folder, ...) cache_path_(folder, (const char*[]){__VA_ARGS__, NULL})

#endif  // PROVIDERS_H_
