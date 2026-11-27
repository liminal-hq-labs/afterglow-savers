# Quill installer integration

How Quill's installers (built by liminal-installer from Quill's `installer/` project) would ship Quill Ghostwriter. This repository does not edit Quill or the installer (SAV-D24). The component names, sizes and wiring are filled in once both editions are built; the fixed points so far:

- **Windows 95 installer:** two optional components, both unselected by default: Ghostwriter for Windows 95 and later (the Win32 `AGQUILL.SCR`, into `%WINDIR%\SYSTEM`) and Ghostwriter for Windows 3.x (`QUILLGW.SCR`, into the Windows directory).
- **Windows 3.x installer:** one optional component, unselected by default: `QUILLGW.SCR` into the Windows directory.
- Neither installer selects Ghostwriter as the screensaver; it appears in the Screen Saver list for the user to find.
