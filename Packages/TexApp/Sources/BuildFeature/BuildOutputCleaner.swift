import Foundation

/// Build → Clean: removes the generated outputs named `stem.ext` under `root`.
public enum BuildOutputCleaner {
    public static func clean(stem: String, extensions: [String], in root: URL) {
        for ext in extensions {
            try? FileManager.default.removeItem(at: root.appendingPathComponent("\(stem).\(ext)"))
        }
    }
}
