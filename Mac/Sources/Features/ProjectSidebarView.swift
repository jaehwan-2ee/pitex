import AppKit
import ProjectFeature
import SwiftUI

extension SidebarSection {
    var titleKey: LocalizedStringKey {
        switch self {
        case .outline: "sidebar.outline"
        case .labels: "sidebar.labels"
        case .bibtex: "sidebar.bibtex"
        }
    }
}

/// Document structure above a separate Project | TODOs switcher.
struct ProjectSidebarView: View {
    @ObservedObject var workspace: WorkspaceModel
    @State private var showingTodos = false
    /// Inline-rename state for the TODOs pane (which row, draft text).
    @State private var editingTodoID: DocumentTodoItem.ID?
    @State private var editingTodoText = ""

    var body: some View {
        VStack(spacing: 0) {
            // Segments clip their labels once the column gets narrow, so
            // fall back to a compact menu picker below the fit width.
            ViewThatFits(in: .horizontal) {
                Picker("sidebar.outline", selection: $workspace.sidebarSection) {
                    ForEach(SidebarSection.allCases) { item in
                        Text(item.titleKey).tag(item)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .controlSize(.small)
                .fixedSize()

                Picker("sidebar.outline", selection: $workspace.sidebarSection) {
                    ForEach(SidebarSection.allCases) { item in
                        Text(item.titleKey).tag(item)
                    }
                }
                .labelsHidden()
                .controlSize(.small)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(6)
            .accessibilityIdentifier("pitex.sidebar.section")

            Divider()

            sectionContent

            Divider()

            VStack(spacing: 0) {
                Picker("sidebar.project", selection: $showingTodos) {
                    Text("sidebar.project").tag(false)
                    Text("sidebar.todos").tag(true)
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                .controlSize(.small)
                .padding(6)
                .accessibilityIdentifier("pitex.sidebar.projectSection")

                if showingTodos {
                    todosPane
                        .frame(minHeight: 120, idealHeight: 180, maxHeight: 280)
                } else {
                    projectSection
                }
            }
        }
        .accessibilityIdentifier("pitex.projectOutline")
    }

    // MARK: - Upper section (document structure)

    @ViewBuilder
    private var sectionContent: some View {
        switch workspace.sidebarSection {
        case .outline:
            outlinePane
        case .labels:
            labelsPane
        case .bibtex:
            bibtexPane
        }
    }

    private var outlinePane: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("sidebar.outline")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Button {
                    workspace.rescanProject()
                } label: {
                    Image(systemName: "arrow.clockwise")
                }
                .buttonStyle(.borderless)
                .help(String(localized: "sidebar.rescan_help"))
            }
            .padding(.horizontal, 10)
            .padding(.top, 8)

            if workspace.outlineItems.isEmpty {
                Text("sidebar.no_outline")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(workspace.outlineItems) { item in
                    Button {
                        workspace.jumpTo(line: item.line, column: 0)
                    } label: {
                        Text(verbatim: item.title)
                            .lineLimit(1)
                            .padding(.leading, CGFloat(item.level) * 10)
                    }
                    .buttonStyle(.plain)
                }
                .listStyle(.plain)
            }
        }
        .frame(maxHeight: .infinity)
        .accessibilityIdentifier("pitex.sidebar.outline")
    }

    private var labelsPane: some View {
        Group {
            if workspace.labelItems.isEmpty {
                ContentUnavailableView(
                    "sidebar.no_labels",
                    systemImage: "tag",
                    description: Text("sidebar.no_labels_detail")
                )
            } else {
                List(workspace.labelItems) { item in
                    Button {
                        workspace.jumpTo(line: item.line, column: 0)
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "tag")
                                .foregroundStyle(.secondary)
                            Text(verbatim: item.name)
                                .lineLimit(1)
                            Spacer()
                            Text(verbatim: "\(item.line)")
                                .font(.caption2)
                                .foregroundStyle(.tertiary)
                        }
                    }
                    .buttonStyle(.plain)
                }
                .listStyle(.plain)
            }
        }
        .frame(maxHeight: .infinity)
        .accessibilityIdentifier("pitex.sidebar.labels")
    }

    private var bibtexPane: some View {
        Group {
            if workspace.bibliographyItems.isEmpty {
                ContentUnavailableView(
                    "sidebar.no_bib",
                    systemImage: "book",
                    description: Text("sidebar.no_bib_detail")
                )
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 1) {
                        ForEach(workspace.bibliographyItems) { item in
                            HStack(spacing: 6) {
                                Image(systemName: "book")
                                    .foregroundStyle(.secondary)
                                VStack(alignment: .leading, spacing: 1) {
                                    Text(verbatim: item.key)
                                        .lineLimit(1)
                                    Text(verbatim: "\(item.type) · \(item.file)")
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                }
                            }
                            .padding(.horizontal, 8)
                            .padding(.vertical, 3)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                    .padding(.horizontal, 4)
                }
            }
        }
        .frame(maxHeight: .infinity)
        .accessibilityIdentifier("pitex.sidebar.bibtex")
    }

    private var todosPane: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("sidebar.todos")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Text(verbatim: "\(workspace.todoItems.count)")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                Spacer()
                Button {
                    workspace.addTodo()
                } label: {
                    Image(systemName: "plus")
                }
                .buttonStyle(.borderless)
                .disabled(!workspace.canAddTodo)
                .help(String(localized: "todos.add_help"))
                .accessibilityIdentifier("pitex.sidebar.todos.add")
            }
            .padding(.horizontal, 10)
            .padding(.top, 8)

            if workspace.todoItems.isEmpty {
                ContentUnavailableView(
                    "sidebar.no_todos",
                    systemImage: "checklist",
                    description: Text("sidebar.no_todos_detail")
                )
            } else {
                List(workspace.todoItems) { item in
                    todoRow(item)
                }
                .listStyle(.plain)
            }
        }
        .frame(maxHeight: .infinity)
        .accessibilityIdentifier("pitex.sidebar.todos")
    }

    private func todoRow(_ item: DocumentTodoItem) -> some View {
        HStack(spacing: 6) {
            Toggle(isOn: Binding(
                get: { item.done },
                set: { _ in workspace.toggleTodo(item) }
            )) {
                EmptyView()
            }
            .toggleStyle(.checkbox)
            .labelsHidden()

            if editingTodoID == item.id {
                TextField("", text: $editingTodoText)
                    .textFieldStyle(.plain)
                    .onSubmit {
                        workspace.renameTodo(item, to: editingTodoText)
                        editingTodoID = nil
                    }
            } else {
                Button {
                    Task { await workspace.openTodo(item) }
                } label: {
                    VStack(alignment: .leading, spacing: 1) {
                        Text(verbatim: item.text)
                            .lineLimit(1)
                            .strikethrough(item.done)
                            .foregroundStyle(item.done ? .secondary : .primary)
                        Text(verbatim: "\(item.file):\(item.line)")
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .contextMenu {
            Button("todos.rename") {
                editingTodoText = item.text
                editingTodoID = item.id
            }
            Button("todos.delete", role: .destructive) {
                workspace.removeTodo(item)
            }
        }
    }

    // MARK: - Project section

    private var projectSection: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("sidebar.project")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Button {
                    workspace.togglePinnedBuildTarget()
                } label: {
                    Image(systemName: workspace.pinnedBuildTarget != nil ? "pin.fill" : "pin")
                }
                .buttonStyle(.borderless)
                .disabled(workspace.activeDocumentURL?.pathExtension.lowercased() != "tex")
                .help(String(localized: "sidebar.pin_help"))
            }
            .padding(.horizontal, 10)
            .padding(.top, 8)
            .padding(.bottom, 4)

            // ScrollView+VStack instead of List: NSTableView marks clicked
            // rows selected (accent wash) even without a selection binding —
            // we want only our own active-file highlight. Only directories
            // collapse; the main document's dependencies stay visible.
            let project = extractOutputPDFs(workspace.projectTree, mainPath: workspace.buildSourceRelativePath() ?? "")
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 1) {
                    ProjectTreeRows(nodes: project.tree) { fileRow($0) }
                }
                .padding(.horizontal, 4)
            }
            .frame(minHeight: 80, idealHeight: 140, maxHeight: 220)

            // The compiled PDF (main-stem .pdf) is pinned just above the
            // project path, set off by a dotted separator — an artifact,
            // not a source row.
            if !project.outputs.isEmpty {
                Rectangle()
                    .stroke(style: StrokeStyle(lineWidth: 1, dash: [2, 3]))
                    .foregroundStyle(.secondary)
                    .frame(height: 1)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 3)
                ForEach(project.outputs) { fileRow($0) }
                    .padding(.horizontal, 4)
                    .padding(.bottom, 4)
            }

            if let path = workspace.projectURL?.path {
                Text(verbatim: path)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
                    .truncationMode(.head)
                    .padding(.horizontal, 10)
                    .padding(.bottom, 8)
            }
        }
    }


    private func fileRow(_ node: ProjectFileNode) -> some View {
        let url = workspace.projectURL?.appendingPathComponent(node.path)
            ?? URL(fileURLWithPath: node.path)
        let isSource = WorkspaceModel.isSourceFile(url)
        return Button {
            if isSource {
                Task { await workspace.activateDocument(url) }
            } else {
                // Figures open in the system viewer, never the text editor.
                _ = NSWorkspace.shared.open(url)
            }
        } label: {
            HStack(spacing: 7) {
                Image(systemName: isSource
                    ? (url.pathExtension.lowercased() == "bib" ? "book" : "doc.text")
                    : (url.pathExtension.lowercased() == "pdf" ? "doc.richtext" : "photo"))
                Text(verbatim: node.name)
                    .lineLimit(1)
                    .truncationMode(.middle)
                Spacer()
                if isSource, url == workspace.pinnedBuildTarget {
                    Image(systemName: "pin.fill")
                        .foregroundStyle(.orange)
                        .imageScale(.small)
                } else if isSource, url == workspace.automaticBuildTarget {
                    Image(systemName: "hammer")
                        .foregroundStyle(.secondary)
                        .imageScale(.small)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                url == workspace.activeDocumentURL
                    ? Color.primary.opacity(0.10)
                    : Color.clear,
                in: RoundedRectangle(cornerRadius: 4)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(node.path)
        .accessibilityIdentifier("pitex.project.file.\(node.path)")
    }

}

/// Files keep their dependency rows visible; only real directories disclose.
private struct ProjectTreeRows<FileRow: View>: View {
    let nodes: [ProjectFileNode]
    let fileRow: (ProjectFileNode) -> FileRow

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            ForEach(nodes) { node in
                if node.isDirectory {
                    ProjectFolderRow(node: node, fileRow: fileRow)
                } else {
                    fileRow(node)
                    if let children = node.children {
                        ProjectTreeRows(nodes: children, fileRow: fileRow)
                            .padding(.leading, 24)
                    }
                }
            }
        }
    }
}

/// A directory row. The whole label (icon, name, blank space) toggles, not
/// just DisclosureGroup's chevron.
private struct ProjectFolderRow<FileRow: View>: View {
    let node: ProjectFileNode
    let fileRow: (ProjectFileNode) -> FileRow
    @State private var isExpanded = false

    var body: some View {
        DisclosureGroup(isExpanded: $isExpanded) {
            ProjectTreeRows(nodes: node.children ?? [], fileRow: fileRow)
                .padding(.leading, 24)
        } label: {
            HStack(spacing: 7) {
                Image(systemName: "folder")
                Text(verbatim: node.name)
                    .lineLimit(1)
                Spacer()
            }
            .padding(.vertical, 3)
            .contentShape(Rectangle())
            .onTapGesture { withAnimation { isExpanded.toggle() } }
            .accessibilityIdentifier("pitex.project.dir.\(node.path)")
        }
    }
}
