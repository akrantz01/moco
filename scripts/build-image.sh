#!/usr/bin/env bash

check_environment_variable() {
  env_set=0

  for variable in $@; do
    if [ -z "${!variable}" ]; then
      echo "environment variable $variable must be set"
      env_set=1
    fi
  done

  [ "$env_set" -eq 1 ] && exit 1
}

# BINARY     - the binary to add to the image
# BASE_IMAGE - the base image to inherit from
# PLATFORM   - the platform the image targets
# MANIFEST   - the manifest to add the image to
# DOCKER_METADATA_OUTPUT_JSON - the output from docker/metadata-action
env_set=0
for variable in BASE_IMAGE BINARY MANIFEST PLATFORM DOCKER_METADATA_OUTPUT_JSON; do
  if [ -z "${!variable}" ]; then
    echo "environment variable $variable must be set"
    env_set=1
  fi
done
[ "$env_set" -eq 1 ] && exit 1

echo binary: $BINARY
echo base image: $BASE_IMAGE
echo platform: $PLATFORM
echo manifest: $MANIFEST

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

container=$(buildah from --platform $PLATFORM $BASE_IMAGE)

buildah config --cmd '[]' --entrypoint '[ "/moco" ]' $container
buildah config $(jq -cr '.labels | to_entries | map("--label \"\(.key)=\(.value)\"") | join(" ")' <<< "$DOCKER_METADATA_OUTPUT_JSON")
buildah copy $container $BINARY /moco

image_id=$(buildah commit --rm --manifest $MANIFEST $container)
buildah tag $image_id $(jq -cr '.tags | join(" ")' <<< "$DOCKER_METADATA_OUTPUT_JSON")

buildah images
buildah inspect $image_id
buildah manifest inspect $MANIFEST

buildah rmi -f $image_id
buildah manifest rm $MANIFEST
