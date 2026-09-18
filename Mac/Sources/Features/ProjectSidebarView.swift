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

/// Left sidebar: a segmented Outline | Labels | BibTeX picker whose upper
/// pane shows structure parsed from the active document, with a fixed
/// "Project" file list and the project path pinned at the bottom.
struct ProjectSidebarView: View {
    @ObservedObject var workspace: WorkspaceModel

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

            projectSection
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

    // MARK: - Project section (always visible)

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
            // we want only our own active-file highlight. OutlineGroup draws
            // the directory disclosure triangles.
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 1) {
                    OutlineGroup(projectTree, children: \.children) { node in
                        if node.isDirectory {
                            HStack(spacing: 7) {
                                Image(systemName: "folder")
                                Text(verbatim: node.name)
                                    .lineLimit(1)
                                Spacer()
                            }
                            .padding(.vertical, 3)
                            .accessibilityIdentifier("pitex.project.dir.\(node.path)")
                        } else {
                            fileRow(node)
                        }
                    }
                }
                .padding(.horizontal, 4)
            }
            .frame(minHeight: 80, idealHeight: 140, maxHeight: 220)

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

    private var projectTree: [ProjectFileNode] {
        buildProjectFileTree(relativePaths: workspace.projectFiles.map { relativeDisplayPath($0) })
    }

    private func fileRow(_ node: ProjectFileNode) -> some View {
        let url = workspace.projectURL?.appendingPathComponent(node.path)
            ?? URL(fileURLWithPath: node.path)
        return Button {
            Task { await workspace.activateDocument(url) }
        } label: {
            HStack(spacing: 7) {
                Image(systemName: url.pathExtension.lowercased() == "bib" ? "book" : "doc.text")
                Text(verbatim: node.name)
                    .lineLimit(1)
                Spacer()
                if url == workspace.pinnedBuildTarget {
                    Image(systemName: "pin.fill")
                        .foregroundStyle(.orange)
                        .imageScale(.small)
                } else if url == workspace.automaticBuildTarget {
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
        .accessibilityIdentifier("pitex.project.file.\(node.path)")
    }

    private func relativeDisplayPath(_ url: URL) -> String {
        guard let root = workspace.projectURL else { return url.lastPathComponent }
        let rootPath = root.standardizedFileURL.path
        let path = url.standardizedFileURL.path
        let prefix = rootPath == "/" ? "/" : rootPath + "/"
        guard path.hasPrefix(prefix) else { return url.lastPathComponent }
        return String(path.dropFirst(prefix.count))
    }
}
