
# Images used in this script are build in CasperLabs/buildenv repo

# This allows make commands without local build environment setup or
# using an OS version other than locally installed.

set -e

docker pull casperlabs/node-build-u1804:latest

# Getting user and group to chown/chgrp target folder from root at end.
# Cannot use the --user trick as cached .cargo in image is owned by root.
command="cd /casper-node/node; cargo build --release; chown -R -f $(id -u):$(id -g) ./target ./target_as ./execution_engine_testing/casper_casper;"
docker run --rm --volume $(pwd):/casper-node casperlabs/node-build-u1804:latest /bin/bash -c "${command}"
