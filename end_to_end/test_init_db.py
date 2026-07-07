from .helper import diaria, vault, Vault


def test_init_creates_a_vault(diaria, vault: Vault):
    """`diaria init` (run by the `vault` fixture) must lay down a complete v1
    vault: the three key files, the version manifest, and the entries dir."""
    assert (vault.dir / "key.key").is_file()
    assert (vault.dir / "key.pub").is_file()
    assert (vault.dir / "key.sym").is_file()
    assert (vault.dir / "manifest.toml").is_file()
    assert vault.entries.is_dir()
