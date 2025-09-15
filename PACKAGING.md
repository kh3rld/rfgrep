Packaging and Deployment
-----------------------

Docker
- Multi-stage Dockerfile is provided at `Dockerfile`. Build with:

```bash
docker build -t rfgrep:latest .
```

Systemd
- Example unit is in `dist/rfgrep.service`. Copy to `/etc/systemd/system/` and enable.

CI
- GitHub Actions workflow at `.github/workflows/ci.yml` runs build, tests, cargo-audit and signs artifacts when `GPG_PRIVATE_KEY` secret is present.

Snap
- `snap/snapcraft.yaml` is included to build a snap package. Use `snapcraft --use-lxd` or remote builders to build snaps.
