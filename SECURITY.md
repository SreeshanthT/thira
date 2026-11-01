# Security policy

## Supported versions

Thira has not had a release yet. Until v1.0, only the latest release (and `main`) receives security fixes.

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub: go to the [Security tab](https://github.com/SreeshanthT/thira/security) and choose **Report a vulnerability**. Do not open a public issue.

Include what you can of:

- the affected version or commit;
- steps to reproduce, ideally with a sample file (media files that crash the parser are especially useful);
- the impact you expect, such as a crash, code execution or data loss.

You will get a reply within 7 days. Once a fix is ready, the advisory is published with credit to you unless you ask otherwise.

## Scope

Thira opens untrusted media and project files, so these are in scope:

- crashes, memory corruption or code execution from crafted media, project or interchange files;
- writes outside the project and cache folders;
- any network access that is not an explicit, user-initiated update check (PRD: no telemetry).

Vulnerabilities in third-party libraries (such as FFmpeg) should also be reported to that project. Tell us too, so we can update.
