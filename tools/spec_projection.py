"""Validate visible normative text against the embedded artifact model.

This checker compares all modeled sections, requirements, acceptance rows and
source notes. Formatting whitespace is ignored; words and order are not.
"""
from __future__ import annotations

from html.parser import HTMLParser
from typing import Any


def normalized(text: str) -> str:
    return ' '.join(text.split())


class Projection(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.sections: dict[str, list[str]] = {}
        self.sources: dict[str, list[str]] = {}
        self.section: str | None = None
        self.source: str | None = None
        self.depth = 0

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attrs = dict(attrs)
        if tag == 'section':
            if self.section is not None:
                self.depth += 1
            elif 'data-panel' in attrs:
                key = attrs.get('id')
                if not key or key in self.sections:
                    raise ValueError('Missing or duplicate projected section')
                self.section, self.depth = key, 1
                self.sections[key] = []
        if tag == 'article' and attrs.get('class') == 'source':
            key = attrs.get('id')
            if not key or key in self.sources:
                raise ValueError('Missing or duplicate projected source')
            self.source = key
            self.sources[key] = []

    def handle_endtag(self, tag: str) -> None:
        if tag == 'section' and self.section is not None:
            self.depth -= 1
            if not self.depth:
                self.section = None
        if tag == 'article':
            self.source = None

    def handle_data(self, text: str) -> None:
        if self.section is not None:
            self.sections[self.section].append(text)
        if self.source is not None:
            self.sources[self.source].append(text)


def expected_section(section: dict[str, Any]) -> str:
    result = [section['title'], section['lead']]
    for block in section['blocks']:
        kind = block['type']
        if kind == 'paragraph':
            result += [block['title'], block['text'], *block.get('sources', [])]
        elif kind == 'table':
            result += block['headers']
            for row in block['rows']:
                result += row
        elif kind == 'requirement':
            result += [f"{block['id']} · {block['priority']}", block['title'],
                       block['text'], 'Acceptance evidence', block['tests']]
        elif kind == 'code':
            result.append(block['text'])
        else:
            raise ValueError('Unsupported canonical block type')
    return normalized(' '.join(result))


def check(html: str, model: dict[str, Any]) -> None:
    projection = Projection()
    projection.feed(html)
    if projection.section is not None:
        raise ValueError('Unclosed projected section')
    wanted = {s['id'] for s in model['sections']} | {'SPEC-SOURCES'}
    if set(projection.sections) != wanted:
        raise ValueError('Projection section set differs from canonical model')
    for section in model['sections']:
        actual = normalized(' '.join(projection.sections[section['id']]))
        if actual != expected_section(section):
            raise ValueError(f"Model/projection mismatch: {section['id']}")
    if set(projection.sources) != {s['id'] for s in model['sources']}:
        raise ValueError('Projection source set differs from canonical model')
    for source in model['sources']:
        expected = normalized(' '.join([f"{source['id']} · primary source",
                                       source['title'], source['note'], source['url']]))
        if normalized(' '.join(projection.sources[source['id']])) != expected:
            raise ValueError(f"Model/projection mismatch: {source['id']}")
