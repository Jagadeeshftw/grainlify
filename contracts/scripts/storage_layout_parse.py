#!/usr/bin/env python3
"""Deterministic parser for `#[contracttype]` layouts in a Rust source file.

Used by:
  * contracts/scripts/gen_storage_layout_types.py (regeneration)
  * scripts/verify-storage-layout-manifest.py          (CI verification)

The parser is intentionally syntax-light: it pins the *declared field order and
encoding types* of every `#[contracttype]` struct/enum in a source file, which
is exactly what changes when a field is added, removed, reordered or retyped.
"""
from __future__ import annotations

import hashlib
import re

_STRUCT_RE = re.compile(r'^\s*pub\s+(struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{', re.M)
_MOD_RE = re.compile(r'\bmod\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{')


def strip_noise(text: str) -> str:
    """Blank out // comments and string literals so brace counting is safe."""
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        if c == '/' and i + 1 < n and text[i + 1] == '/':
            j = text.find('\n', i)
            j = n if j == -1 else j
            out.append(' ' * (j - i))
            i = j
        elif c == '"':
            j = i + 1
            while j < n:
                if text[j] == '\\':
                    j += 2
                    continue
                if text[j] == '"':
                    j += 1
                    break
                j += 1
            out.append(' ' * (j - i))
            i = j
        else:
            out.append(c)
            i += 1
    return ''.join(out)


def _match_brace(text: str, open_idx: int) -> int:
    depth = 0
    j = open_idx
    while j < len(text):
        if text[j] == '{':
            depth += 1
        elif text[j] == '}':
            depth -= 1
            if depth == 0:
                return j
        j += 1
    raise ValueError("unbalanced braces")


def _module_ranges(clean: str):
    """Return [(open_brace, name, close_brace)] for every `mod name { ... }`."""
    out = []
    for m in _MOD_RE.finditer(clean):
        open_idx = clean.find('{', m.end() - 1)
        if open_idx == -1:
            continue
        close = _match_brace(clean, open_idx)
        out.append((open_idx, m.group(1), close))
    return out


def _module_prefix(opens, pos: int) -> str:
    names = [name for (o, name, close) in opens if o < pos < close]
    return '::'.join(names)


def _split_top_level(body: str):
    parts, cur, depth = [], [], 0
    for ch in body:
        if ch in '<([{':
            depth += 1
        elif ch in '>)]}':
            depth -= 1
        if ch == ',' and depth == 0:
            parts.append(''.join(cur))
            cur = []
        else:
            cur.append(ch)
    parts.append(''.join(cur))
    return parts


def _norm(raw: str) -> str:
    return ' '.join(raw.split())


def _struct_fields(body: str):
    fields = []
    for raw in _split_top_level(body):
        piece = re.sub(r'#\[[^\]]*\]', '', raw).strip()
        m = re.match(r'^pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.+)$', piece, re.S)
        if m:
            fields.append([m.group(1), _norm(m.group(2).rstrip(',').strip())])
    return fields


def _enum_variants(body: str):
    variants = []
    for raw in _split_top_level(body):
        piece = re.sub(r'#\[[^\]]*\]', '', raw).strip()
        # drop explicit discriminants (`Variant = 42`) from the recorded name
        piece = re.sub(r'\s*=\s*[^,]+$', '', piece).strip()
        if not piece:
            continue
        m = re.match(r'^([A-Za-z_][A-Za-z0-9_]*)\s*(?:\((.*)\))?$', piece, re.S)
        if m:
            payload = m.group(2)
            enc = m.group(1)
            if payload:
                enc += "(" + ", ".join(_norm(p) for p in _split_top_level(payload)) + ")"
            variants.append(enc)
    return variants


def _has_contracttype_attr(text: str, decl_line_start: int) -> bool:
    """True when the contiguous attribute/doc block above a decl has #[contracttype]."""
    lines = text[:decl_line_start].splitlines()
    for line in reversed(lines[-12:]):
        s = line.strip()
        if s == '#[contracttype]':
            return True
        if s == '' or s.startswith('#[') or s.startswith('///') or s.startswith('//!') or s.startswith('//'):
            continue
        return False
    return False


def parse_contracttype_types(path):
    text = path.read_text(encoding="utf-8")
    clean = strip_noise(text)
    opens = _module_ranges(clean)
    results = []
    for m in _STRUCT_RE.finditer(clean):
        line_start = text.rfind('\n', 0, m.start()) + 1
        if not _has_contracttype_attr(text, line_start):
            continue
        kind, name = m.group(1), m.group(2)
        brace = clean.find('{', m.start())
        body = clean[brace + 1:_match_brace(clean, brace)]
        prefix = _module_prefix(opens, m.start())
        encoding = _struct_fields(body) if kind == 'struct' else _enum_variants(body)
        results.append({
            "type": f"{prefix}::{name}" if prefix else name,
            "kind": kind,
            "encoding": encoding,
        })
    return results


def load_serialization_goldens(path) -> dict:
    """Parse `pub const EXPECTED: &[(&str, &str)]` from serialization_goldens.rs."""
    text = path.read_text(encoding="utf-8")
    text = re.sub(r'//[^\n]*', '', text)
    entry_re = re.compile(r'\(\s*"([^"]+)"\s*,\s*(.*?)\s*\)\s*,', re.S)
    goldens = {}
    for name, rhs in entry_re.findall(text):
        rhs = rhs.replace('concat!', '')
        hex_parts = re.findall(r'"([0-9a-fA-F]+)"', rhs)
        if hex_parts:
            goldens[name] = ''.join(hex_parts)
    return goldens


def sha256_hex(value: str) -> str:
    return hashlib.sha256(value.encode()).hexdigest()
