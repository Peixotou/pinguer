# Security Policy

## Supported versions

Pinguer is pre-release software (0.1.0-dev). Security fixes are applied to
the `main` branch only.

## Reporting a vulnerability

Please **do not** open public issues for security vulnerabilities.

Report vulnerabilities privately through GitHub: open the repository's
**Security** tab and choose **Report a vulnerability** (private security
advisory).

Please include:

* a description of the issue and its impact;
* steps or a minimal input to reproduce it;
* the affected version or commit.

Relevant issues include, for example, compiler crashes or hangs on crafted
input, diagnostics that can be abused to manipulate terminal output, and,
once code generation exists, miscompilations that break the language's safety
guarantees.

## Security model

The language's security goals and current guarantees are described in
[specifications/SECURITY_MODEL.md](specifications/SECURITY_MODEL.md).
