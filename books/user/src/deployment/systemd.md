# systemd
`cuprated` can be ran as a `systemd` service, the below are commands to setup a relatively hardened deployment.

```bash
# Create the `cuprate` user
sudo useradd --system --shell /sbin/nologin --home-dir /home/cuprate cuprate

# Move `cuprated` and the config file
# into the appropriate location.
mv cuprated Cuprated.toml /home/cuprate/

# Move the service file to the appropriate location.
sudo mv cuprated.service /etc/systemd/system/

# Start the `cuprated` service.
sudo systemctl daemon-reload
sudo systemctl start cuprated

# (Optional) start `cuprated` upon boot.
sudo systemctl enable cuprated
```

A relatively hardened [`systemd` service file](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html) for `cuprated`:

```properties
{{#include ../../../../binaries/cuprated/cuprated.service}}
```