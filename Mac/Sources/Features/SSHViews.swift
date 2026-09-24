import AppKit
import RemoteCore
import SwiftUI

/// Settings → SSH: the devices "Open via SSH" can open folders on.
struct SSHSettingsPane: View {
    @ObservedObject var store: SettingsStore
    @State private var adding = false
    @State private var checks: [UUID: Check] = [:]

    enum Check: Equatable {
        case running
        case passed(String)
        case failed(String)
    }

    var body: some View {
        Form {
            Section {
                if store.sshConnections.isEmpty {
                    Text("settings.ssh.empty")
                        .foregroundStyle(.secondary)
                }
                ForEach(store.sshConnections) { connection in
                    row(connection)
                }
                Button("settings.ssh.add") { adding = true }
                    .accessibilityIdentifier("pitex.settings.ssh.add")
            } header: {
                Text("settings.ssh.connections")
            } footer: {
                Text("settings.ssh.note")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .sheet(isPresented: $adding) {
            AddSSHConnectionSheet(store: store)
        }
    }

    private func row(_ connection: SSHConnection) -> some View {
        HStack(spacing: 10) {
            Image(systemName: "desktopcomputer")
                .foregroundStyle(.secondary)
            VStack(alignment: .leading, spacing: 2) {
                Text(verbatim: connection.name)
                Text(verbatim: summary(connection))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                switch checks[connection.id] {
                case .running:
                    ProgressView().controlSize(.small)
                case let .passed(detail):
                    Label(detail, systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundStyle(.green)
                case let .failed(reason):
                    Label(reason, systemImage: "xmark.octagon.fill")
                        .font(.caption)
                        .foregroundStyle(.red)
                        .lineLimit(3)
                case nil:
                    EmptyView()
                }
            }
            Spacer()
            Button("settings.ssh.test") { test(connection) }
                .disabled(checks[connection.id] == .running)
            Button(role: .destructive) {
                store.sshConnections.removeAll { $0.id == connection.id }
            } label: {
                Image(systemName: "trash")
            }
            .buttonStyle(.borderless)
            .help(String(localized: "settings.ssh.remove"))
        }
        .accessibilityIdentifier("pitex.settings.ssh.connection")
    }

    private func summary(_ connection: SSHConnection) -> String {
        var text = connection.destination
        if let user = connection.user, !user.isEmpty { text = "\(user)@\(text)" }
        if let port = connection.port { text += ":\(port)" }
        return text
    }

    /// Connects once (key auth, host key) and looks for latexmk through the
    /// remote login shell — what a remote build will use.
    private func test(_ connection: SSHConnection) {
        checks[connection.id] = .running
        Task {
            let client = SSHClient(connection: connection)
            do {
                let home = try await client.check()
                let latexmk = try? await client.which("latexmk")
                checks[connection.id] = .passed(latexmk.map {
                    String(format: String(localized: "settings.ssh.test_tex"), $0)
                } ?? String(format: String(localized: "settings.ssh.test_no_tex"), home))
            } catch {
                checks[connection.id] = .failed(error.localizedDescription)
            }
        }
    }
}

/// "Add SSH Connection": hosts from ~/.ssh/config, or entered by hand.
struct AddSSHConnectionSheet: View {
    @ObservedObject var store: SettingsStore
    var onAdded: ((SSHConnection) -> Void)?
    @Environment(\.dismiss) private var dismiss
    @State private var discovered: [SSHHostEntry] = []
    @State private var selection: String?
    @State private var manual = false
    @State private var name = ""
    @State private var host = ""
    @State private var user = ""
    @State private var port = ""
    @State private var identityFile = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            SheetHeader(title: "ssh.add.title") { dismiss() }
            Group {
                if manual { manualForm } else { hostList }
            }
            .frame(maxWidth: .infinity, minHeight: 260, maxHeight: 260)
            .background(RoundedRectangle(cornerRadius: 12).fill(Color(nsColor: .textBackgroundColor)))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.quaternary))
            if let problem {
                Text(verbatim: problem)
                    .font(.caption)
                    .foregroundStyle(.red)
            }
            HStack {
                Button {
                    manual.toggle()
                } label: {
                    Label(manual ? LocalizedStringKey("ssh.add.from_config") : "ssh.add.manual",
                          systemImage: manual ? "list.bullet" : "square.and.pencil")
                }
                .accessibilityIdentifier("pitex.ssh.add.manual")
                if !manual {
                    Button { reload() } label: { Image(systemName: "arrow.clockwise") }
                        .help(String(localized: "ssh.add.rescan"))
                }
                Spacer()
                Button("ssh.add.confirm") { add() }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
                    .disabled(pending == nil)
                    .accessibilityIdentifier("pitex.ssh.add.confirm")
            }
        }
        .padding(20)
        .frame(width: 540)
        .onAppear(perform: reload)
    }

    /// Config hosts not saved yet.
    private var available: [SSHHostEntry] {
        let saved = Set(store.sshConnections.map(\.destination))
        return discovered.filter { !saved.contains($0.alias) }
    }

    @ViewBuilder
    private var hostList: some View {
        if available.isEmpty {
            Text("ssh.add.none_found")
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else {
            List(available, id: \.alias, selection: $selection) { entry in
                HStack(spacing: 10) {
                    Image(systemName: "desktopcomputer").foregroundStyle(.secondary)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(verbatim: entry.alias)
                        Text(verbatim: entry.summary)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                .tag(entry.alias)
            }
            .scrollContentBackground(.hidden)
            .accessibilityIdentifier("pitex.ssh.add.hosts")
        }
    }

    private var manualForm: some View {
        Form {
            TextField("ssh.field.name", text: $name, prompt: Text(verbatim: "Mac mini"))
            TextField("ssh.field.host", text: $host, prompt: Text(verbatim: "mac-mini.local"))
            TextField("ssh.field.user", text: $user, prompt: Text("ssh.field.optional"))
            TextField("ssh.field.port", text: $port, prompt: Text(verbatim: "22"))
            HStack {
                TextField("ssh.field.key", text: $identityFile, prompt: Text("ssh.field.optional"))
                Button("ssh.field.choose") { chooseKey() }
            }
        }
        .formStyle(.grouped)
        .scrollContentBackground(.hidden)
    }

    private var manualConnection: SSHConnection {
        let trimmedHost = host.trimmingCharacters(in: .whitespaces)
        let trimmedName = name.trimmingCharacters(in: .whitespaces)
        let trimmedUser = user.trimmingCharacters(in: .whitespaces)
        let trimmedKey = identityFile.trimmingCharacters(in: .whitespaces)
        return SSHConnection(
            name: trimmedName.isEmpty ? trimmedHost : trimmedName,
            destination: trimmedHost,
            user: trimmedUser.isEmpty ? nil : trimmedUser,
            port: Int(port.trimmingCharacters(in: .whitespaces)),
            identityFile: trimmedKey.isEmpty ? nil : trimmedKey
        )
    }

    private var pending: SSHConnection? {
        if manual {
            let connection = manualConnection
            guard connection.validationError == nil,
                  port.trimmingCharacters(in: .whitespaces).isEmpty || connection.port != nil else { return nil }
            return connection
        }
        return available.first { $0.alias == selection }.map(SSHConnection.init(configHost:))
    }

    private var problem: String? {
        guard manual, !host.isEmpty else { return nil }
        if !port.trimmingCharacters(in: .whitespaces).isEmpty, Int(port.trimmingCharacters(in: .whitespaces)) == nil {
            return String(localized: "ssh.field.port_invalid")
        }
        return manualConnection.validationError
    }

    private func reload() {
        discovered = SSHConfigParser.loadHosts()
        if selection == nil { selection = available.first?.alias }
    }

    private func chooseKey() {
        let panel = NSOpenPanel()
        panel.showsHiddenFiles = true
        panel.directoryURL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".ssh")
        panel.canChooseDirectories = false
        if panel.runModal() == .OK, let url = panel.url { identityFile = url.path }
    }

    private func add() {
        guard let connection = pending else { return }
        store.sshConnections.append(connection)
        onAdded?(connection)
        dismiss()
    }
}

/// File → "Open via SSH…": pick a device, then a folder on it.
struct OpenViaSSHSheet: View {
    @ObservedObject var workspace: WorkspaceModel
    @ObservedObject var store: SettingsStore
    @Environment(\.dismiss) private var dismiss
    @State private var connectionID: UUID?
    @State private var folder: String?
    @State private var browsing = false
    @State private var adding = false

    private var connection: SSHConnection? {
        store.sshConnections.first { $0.id == connectionID } ?? store.sshConnections.first
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            SheetHeader(title: "ssh.open.title") { dismiss() }
            Text("ssh.open.folder")
                .font(.headline)
            VStack(spacing: 12) {
                if let connection {
                    Menu {
                        ForEach(store.sshConnections) { candidate in
                            Button(candidate.name) {
                                connectionID = candidate.id
                                folder = nil
                            }
                        }
                        Divider()
                        Button("ssh.open.add_connection") { adding = true }
                    } label: {
                        Text(verbatim: String(format: String(localized: "ssh.open.on_device"), connection.name))
                    }
                    .menuStyle(.borderlessButton)
                    .fixedSize()
                    .accessibilityIdentifier("pitex.ssh.open.device")
                    if let folder {
                        HStack(spacing: 8) {
                            Image(systemName: "folder")
                            Text(verbatim: folder)
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Button("ssh.open.change") { browsing = true }
                        }
                        .padding(.horizontal, 12)
                    } else {
                        Button { browsing = true } label: {
                            Label("ssh.open.choose", systemImage: "folder.badge.plus")
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.large)
                        .accessibilityIdentifier("pitex.ssh.open.choose")
                    }
                } else {
                    Text("ssh.open.no_connections")
                        .foregroundStyle(.secondary)
                    Button("ssh.open.add_connection") { adding = true }
                        .buttonStyle(.bordered)
                }
            }
            .frame(maxWidth: .infinity, minHeight: 150)
            .background(RoundedRectangle(cornerRadius: 14).fill(Color(nsColor: .textBackgroundColor)))
            .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(.quaternary))
            Text("ssh.open.note")
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            HStack {
                Spacer()
                Button("ssh.open.cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button("ssh.open.confirm") { open() }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
                    .disabled(connection == nil || folder == nil)
                    .accessibilityIdentifier("pitex.ssh.open.confirm")
            }
        }
        .padding(22)
        .frame(width: 560)
        .onAppear {
            if connectionID == nil { connectionID = store.lastSSHConnectionID ?? store.sshConnections.first?.id }
        }
        .sheet(isPresented: $browsing) {
            if let connection {
                RemoteFolderBrowser(connection: connection, initialPath: folder ?? "") { folder = $0 }
            }
        }
        .sheet(isPresented: $adding) {
            AddSSHConnectionSheet(store: store) { added in
                connectionID = added.id
                folder = nil
            }
        }
    }

    private func open() {
        guard let connection, let folder else { return }
        store.lastSSHConnectionID = connection.id
        dismiss()
        workspace.openRemote(RemoteProject(connection: connection, remoteRoot: folder))
    }
}

/// Folder picker for a remote device: path field with an up button, the
/// current folder's subfolders (click to enter), TeX/Markdown files shown
/// for orientation, and "Use Folder" for the folder being shown.
struct RemoteFolderBrowser: View {
    let connection: SSHConnection
    let initialPath: String
    let onChoose: (String) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var pathField = ""
    @State private var listing: RemoteDirectoryListing?
    @State private var loading = false
    @State private var error: String?
    @State private var task: Task<Void, Never>?

    private static let hintExtensions: Set<String> = ["tex", "bib", "md", "markdown", "sty", "cls"]

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            SheetHeader(title: "ssh.browse.title") { dismiss() }
            HStack(spacing: 8) {
                Button { go(parentPath) } label: { Image(systemName: "arrow.up") }
                    .disabled(listing == nil || listing?.path == "/")
                    .help(String(localized: "ssh.browse.up"))
                    .accessibilityIdentifier("pitex.ssh.browse.up")
                TextField("", text: $pathField)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { go(pathField) }
                    .accessibilityIdentifier("pitex.ssh.browse.path")
            }
            Button("ssh.browse.use_current") { choose() }
                .buttonStyle(.link)
                .disabled(listing == nil)
            ZStack {
                if let listing {
                    List {
                        ForEach(listing.folders, id: \.self) { name in
                            Button { go((listing.path as NSString).appendingPathComponent(name)) } label: {
                                Label(name, systemImage: "folder")
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                        }
                        ForEach(listing.files.filter { Self.hintExtensions.contains(($0 as NSString).pathExtension.lowercased()) },
                                id: \.self) { name in
                            Label(name, systemImage: "doc.text")
                                .foregroundStyle(.secondary)
                        }
                        if listing.folders.isEmpty && listing.files.isEmpty {
                            Text("ssh.browse.empty").foregroundStyle(.secondary)
                        }
                    }
                    .scrollContentBackground(.hidden)
                    .opacity(loading ? 0.4 : 1)
                    .accessibilityIdentifier("pitex.ssh.browse.list")
                }
                if loading {
                    ProgressView()
                } else if let error {
                    Text(verbatim: error)
                        .foregroundStyle(.red)
                        .multilineTextAlignment(.center)
                        .padding()
                }
            }
            .frame(maxWidth: .infinity, minHeight: 300, maxHeight: 300)
            .background(RoundedRectangle(cornerRadius: 12).fill(Color(nsColor: .textBackgroundColor)))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(.quaternary))
            HStack {
                Spacer()
                Button("ssh.open.cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button("ssh.browse.use_folder") { choose() }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.defaultAction)
                    .disabled(listing == nil || loading)
                    .accessibilityIdentifier("pitex.ssh.browse.useFolder")
            }
        }
        .padding(20)
        .frame(width: 540)
        .onAppear { go(initialPath) }
        .onDisappear { task?.cancel() }
    }

    private var parentPath: String {
        guard let path = listing?.path else { return "" }
        return (path as NSString).deletingLastPathComponent
    }

    private func go(_ path: String) {
        task?.cancel()
        loading = true
        error = nil
        let client = SSHClient(connection: connection)
        task = Task {
            do {
                let result = try await client.listDirectory(path)
                guard !Task.isCancelled else { return }
                listing = result
                pathField = result.path
            } catch {
                guard !Task.isCancelled else { return }
                self.error = error.localizedDescription
                if listing != nil { pathField = listing?.path ?? pathField }
            }
            loading = false
        }
    }

    private func choose() {
        guard let path = listing?.path else { return }
        onChoose(path)
        dismiss()
    }
}

/// Title + close button used by the SSH sheets.
private struct SheetHeader: View {
    let title: LocalizedStringKey
    let close: () -> Void

    var body: some View {
        HStack {
            Text(title)
                .font(.title2.weight(.semibold))
            Spacer()
            Button(action: close) { Image(systemName: "xmark") }
                .buttonStyle(.borderless)
                .help(String(localized: "ssh.close"))
        }
    }
}
