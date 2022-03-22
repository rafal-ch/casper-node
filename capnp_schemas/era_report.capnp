@0xc76be4affcd1f1f1;

using import "public_key.capnp".UserPublicKey;
using import "map.capnp".RewardsMap;

struct EraReport {
  equivocators @0 :List(UserPublicKey);
  rewards @1 :RewardsMap(UserPublicKey);
  inactiveValidators @2 :List(UserPublicKey);
}