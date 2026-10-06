# Pi AI assistant

[All features](../../README.md) · [Main README](../../../README.md)

The bottom console contains the **pi** coding agent and a native chat interface.
Pitex manages a Pi **1.0.2** runtime.
Pitex updates an older runtime when the application starts.
Select **Settings → AI → Agent update** to install the latest stable Pi version.
The assistant shows streamed text, thinking and tool-call rows, a model selector, and document attachments.
The assistant can edit project files.
Each proposed edit applies to a specified file revision.
You can accept or reject the edit.

The assistant includes these eight skills:

| Skill | Function |
|---|---|
| Citation Management | Verifies DOI metadata and repairs BibTeX entries. |
| Scientific Writing | Revises scientific sections while preserving results and evidence. |
| Submission Preflight | Checks source dependencies, references, citations, logs and optional ChkTeX warnings. |
| [Humanizer](https://github.com/blader/humanizer) | Changes AI-style text and keeps its meaning and the author's style. |
| [SciSpace](https://scispace.com/) | Finds academic papers, abstracts, and DOI links. Helps prepare literature reviews and BibTeX entries. |
| LaTeX Compile | Compiles a TeX project with an available TeX runtime. |
| LaTeX Doctor | Checks TeX tools and finds missing components. |
| TeX Live Runtime Installer | Finds existing TeX installations and helps install a TeX Live runtime when necessary. |
