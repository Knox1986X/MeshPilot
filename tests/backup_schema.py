"""Проверка структуры проекта формата, не шифрование. Нужен jsonschema 4.25.1."""
import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator

SCHEMAS = Path(__file__).resolve().parents[1] / "docs/schemas"


def validator(name):
    schema = json.loads((SCHEMAS / name).read_text())
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)


class BackupFormat(unittest.TestCase):
    def test_payload_is_an_allowlist_not_an_execution_plan(self):
        check = validator("profile-payload-v1.schema.json")
        example = {"schemaVersion": 1, "profiles": [], "preferences": {"language": "ru", "theme": "dark"}}
        self.assertTrue(check.is_valid(example))
        for extra in ({"schemaVersion": 2}, {"postImport": True}, {"key": None},
                      {"profiles": [{"networkClass": "homeWifi", "command": None}]}):
            self.assertFalse(check.is_valid(example | extra))

    def test_envelope_version_algorithm_and_sizes(self):
        check = validator("backup-v1.schema.json")
        # Синтаксический образец: не результат AEAD, не пригоден для восстановления.
        envelope = {"format": "MeshPilot", "version": 1, "algorithm": "XChaCha20-Poly1305-IETF",
                    "streamId": "10" * 16, "revisionId": "20" * 16, "keyId": "30" * 16,
                    "parents": [], "nonce": "A" * 32, "ciphertext": "A" * 24}
        self.assertTrue(check.is_valid(envelope))
        for extra in ({"version": 0}, {"algorithm": "none"}, {"nonce": "AA"},
                      {"parents": ["40" * 16] * 2}, {"path": "/"}, {"ciphertext": ""}):
            self.assertFalse(check.is_valid(envelope | extra))


if __name__ == "__main__":
    unittest.main()
