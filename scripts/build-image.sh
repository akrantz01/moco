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
# LABELS     - labels to add to the image
# TAGS       - a list of tagged images
# PLATFORM   - the platform the image targets
# MANIFEST   - the manifest to add the image to
env_set=0
for variable in BASE_IMAGE BINARY MANIFEST PLATFORM TAGS; do
  if [ -z "${!variable}" ]; then
    echo "environment variable $variable must be set"
    env_set=1
  fi
done
[ "$env_set" -eq 1 ] && exit 1

echo binary: $BINARY
echo base image: $BASE_IMAGE
echo labels: $LABELS
echo tags: $TAGS
echo platform: $PLATFORM
echo manifest: $MANIFEST

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

IFS=' ' read -r -a LABELS <<< "$LABELS"
IFS=' ' read -r -a TAGS <<< "$TAGS"

container=$(buildah from --platform $PLATFORM $BASE_IMAGE)

buildah config --cmd '[]' --entrypoint '[ "/moco" ]' $container
buildah config $(printf -- "--label '%s' " "${LABELS[@]}")
buildah copy $container $BINARY /moco

image_id=$(buildah commit --rm --manifest $MANIFEST $container)
buildah tag $image_id "${TAGS[@]}"

buildah images
buildah inspect $image_id
buildah manifest inspect $MANIFEST

buildah rmi -f $image_id
buildah manifest rm $MANIFEST
