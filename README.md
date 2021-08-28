# Serial2TCP_RS: Serial port to TCP relay for Windows

## Why
Currently, Windows Subsystem for Linux (WSL) 2 does not support accessing physical serial (COM) ports on the system. That sucks! The good news, however, is that network communication with the host system <-> WSL2 is possible. This little program allows you to relay a physical port to a TCP socket, such that it can be re-converted to a (virtual) serial port on the Linux side using `socat`. This project attempts to leverage blocking reads to avoid unnecessary background work on the CPU.

## Building
Use `git submodule update` to checkout the `serialport-rs` dependency. There is a patch that must be applied to it at `vendor/serialport_overlapped.patch`. This allows overlapped I/O to be used which enables simultaneous read/write to the serial port. The patch also activates the "between characters" timeout so the read buffer doesn't have to fill up entirely before the call returns. Currently there seems to be some deliberation among the `serialport-rs` contributors on whether to activate overlapped I/O: https://gitlab.com/susurrus/serialport-rs/-/merge_requests/91, so perhaps this patch could be removed in the future.

## Use
The preferred method is to define a .ini config file with the ports to be relayed. Each port starts as a `[relay]` subsection. E.g:
```
[relay]
com="COM4"
host="172.26.192.1:6000"

[relay]
com="COM5"
baud="9600"
host="172.26.192.1:6001"
```
`Serial2Tcp_RS.exe --config <ini file>.ini`

One can also launch the relay using command line arguments:
`Serial2Tcp_RS.exe --host="172.26.192.1:6000" --baud="115200" --comport="COM4"

From WSL2, one can then launch `socat` to complete the bridge. E.g:
`sudo socat pty,link=dev/ttyUSB0,raw,nonblock tcp:172.26.192.1:6000`

## Security
It goes without saying that there is absolutely no authentication/security provided by this relay. Be mindful of who has access to your binding :)
