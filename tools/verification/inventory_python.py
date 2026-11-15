"""Python unittest source registration, retaining concrete inherited case names."""
import ast


def base_name(base):
    if isinstance(base, ast.Name):
        return base.id
    return base.attr if isinstance(base, ast.Attribute) else ''


def own_methods(item):
    return {method.name for method in item.body
            if isinstance(method, (ast.FunctionDef, ast.AsyncFunctionDef))
            and method.name.startswith('test_')}


def parent_methods(classes, base, trail):
    name = base_name(base)
    if name == 'TestCase':
        return True, set()
    if name in classes:
        return python_methods(classes, name, trail)
    return False, set()


def python_methods(classes, name, trail=()):
    if name in trail:
        raise ValueError(f'Recursive Python test inheritance: {name}')
    item = classes[name]
    parents = [parent_methods(classes, base, (*trail, name)) for base in item.bases]
    methods = own_methods(item)
    for _, inherited in parents:
        methods.update(inherited)
    return any(runnable for runnable, _ in parents), methods


def class_tests(classes, name):
    runnable, methods = python_methods(classes, name)
    if runnable:
        yield from (f'{name}.{method}' for method in sorted(methods))


def python_classes(path):
    tree = ast.parse(path.read_text())
    return {item.name: item for item in tree.body if isinstance(item, ast.ClassDef)}


def python_file(root, path):
    classes = python_classes(path)
    names = (name for item in classes for name in class_tests(classes, item))
    for name in names:
        yield dict(id=f'{path.relative_to(root)}::{name}', runner='python',
                   file=str(path.relative_to(root)), test_name=f'{path.stem}.{name}', ignored=False)


def python_tests(root):
    for directory in ('tests', 'tools'):
        for path in sorted((root / directory).glob('test_*.py')):
            yield from python_file(root, path)
