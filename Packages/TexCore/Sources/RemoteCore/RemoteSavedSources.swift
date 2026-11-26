import Foundation

extension RemoteSyncRules {
    /// The SHA-256 of each open source an SSH save must find on the device
    /// afterwards. A forced close must never claim that sources excluded
    /// from the mirror protocol reached the device, so an open source that
    /// is not a readable regular file under the size limit throws, unless it
    /// is clean and gone.
    public static func savedSourceHashes(
        root: URL,
        openSources: [(path: String, isClean: Bool)]
    ) throws -> [String: String] {
        var hashes: [String: String] = [:]
        for source in openSources {
            let path = source.path
            let file = root.appendingPathComponent(path)
            // A pull deleted this clean source: there is nothing to upload or verify.
            var info = stat()
            if source.isClean, !isExcludedPath(path), lstat(file.path, &info) != 0, errno == ENOENT { continue }
            guard !isExcludedPath(path),
                  let values = try? file.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
                  values.isRegularFile == true, values.isSymbolicLink != true,
                  let size = values.fileSize, size < maximumFileSize,
                  let hash = SHA256.hex(ofFile: file) else {
                throw SSHError.remote(status: -1, message: "The file \(path) cannot be saved to the SSH device. Sources must be readable regular files smaller than 50 MB outside excluded folders.")
            }
            hashes[path] = hash
        }
        return hashes
    }
}
