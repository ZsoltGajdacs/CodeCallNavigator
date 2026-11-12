"""Validate the CLI generation attached to a derived presentation cache."""
import json


def validate_presentation_cache(rows):
    session = cli_session(rows.get('session'))
    if session is None:
        return
    generation = session['graph_context'].get('generation')
    require_cache(canonical_generation(generation),
                  'Profiling cache needs a pinned CLI graph generation; open and save the workspace')
    marker = checked_dict(session.get('presentation_cache'),
                          'CLI presentation cache is missing; open and save the workspace')
    require_cache(marker.get('cache_generation'), 'CLI presentation cache has no cached generation')
    identity = (marker.get('source_generation'), marker.get('cache_generation'))
    require_cache(identity == (generation, rows.get('active')),
                  'CLI presentation cache differs from the saved graph context; reopen the workspace')


def cli_session(raw):
    if raw is None:
        return None
    session = checked_dict(json.loads(raw), 'Profiling cache needs saved workspace metadata')
    context = session.get('graph_context')
    if context is None:
        return None  # Legacy immutable copied caches remain valid profiling fixtures.
    checked_dict(context, 'Profiling cache has invalid CLI graph context')
    return session


def canonical_generation(value):
    return isinstance(value, str) and len(value) == 64 and all(c in '0123456789abcdef' for c in value)


def checked_dict(value, message):
    require_cache(isinstance(value, dict), message)
    return value


def require_cache(condition, message):
    if not condition:
        raise ValueError(message)
