# keepy-uppy

Keep your MacBook running with the lid closed — no external display, no admin password.
Music keeps playing, builds keep compiling, agents keep working. The internal display turns off as usual.

Free and open source (GPL-3.0). macOS 14+. Windows/Linux planned.

## Status

Early development. See `docs/mechanism.md` for how it works.

## Safety

A Mac that can't sleep with its lid closed can overheat in a bag. keepy-uppy disarms itself on
quit, below a configurable battery threshold, and when the system reports serious thermal pressure.
