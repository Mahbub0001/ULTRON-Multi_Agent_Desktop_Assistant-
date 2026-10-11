"""Evidence produced by a native adapter, preserved through string-based handlers."""

class VerifiedResult(str):
    def __new__(cls, text, evidence):
        value = super().__new__(cls, text)
        value.evidence = evidence
        return value
