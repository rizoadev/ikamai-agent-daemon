class IadError(Exception):
    """Base SDK error."""

class TaskCancelledError(IadError):
    """Raised when a task is cancelled by the daemon."""
