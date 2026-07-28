# Threshold Monitor Boundary Fuzzing

The threshold monitor starts a new window when `timestamp >= window_start + time_window_secs`.
Its property test generates ordered success/failure sequences at `end - 1`, `end`, and `end + 1`.

For every sequence, the test compares `check_thresholds` with an independent current-window model.
It asserts that the breaker opens exactly when the current window contains at least the configured
number of failures. The test uses `proptest`, so a failing sequence is automatically shrunk to a
small boundary-focused reproducer.

The generated inputs contain no token amounts and exercise no authorization paths. This keeps the
security assertion focused on the monitor's window-membership boundary: failures before a rotation
must not leak into the new window, while failures immediately before it must remain counted.
