import Foundation

/// Build → Clean: removes the generated outputs named `stem.ext` under `root`.
public enum BuildOutputCleaner {
    public static func clean(stem: String, extensions: [String], in root: URL) {
        for ext in extensions {
            let path = root.appendingPathComponent("\(stem).\(ext)").path
            // Only a regular file is a build output; a directory or link with an output name is the user's.
            var info = stat()
            guard lstat(path, &info) == 0, info.st_mode & S_IFMT == S_IFREG else { continue }
            unlink(path)
        }
    }
}
