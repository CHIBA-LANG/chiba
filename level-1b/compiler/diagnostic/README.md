# level-1b compiler diagnostic

Diagnostics must be deterministic across file system traversal order, include
path order, and future parallel execution.

Each diagnostic carries a stable code, severity, primary span, optional notes,
and optional fix text. User-visible wording should avoid exposing temporary
level-0 implementation details.
