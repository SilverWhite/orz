"""Minimal assurance package stub for the Windows S4 run environment.

The canonical package (D:/CLI/assurance) eagerly imports heavy modules that
the sandbox run path does not need.  This run-environment copy keeps only the
run path (windows_sandbox, contracts, errors, utils) importable.
"""
