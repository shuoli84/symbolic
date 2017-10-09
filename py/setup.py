import os
import re
from setuptools import setup, find_packages


_version_re = re.compile(r'^version\s*=\s*"(.*?)"\s*$(?m)')


DEBUG_BUILD = os.environ.get('SYMBOLIC_DEBUG') == '1'


with open('../Cargo.toml') as f:
    version = _version_re.search(f.read()).group(1)


def build_native(spec):
    target = DEBUG_BUILD and 'debug' or 'release'

    # Step 1: build the rust library
    build = spec.add_external_build(
        cmd=['cargo', 'build', '--' + target],
        path='../cabi'
    )

    spec.add_cffi_module(
        module_path='symbolic._lowlevel',
        dylib=lambda: build.find_dylib('symbolic', in_path='target/%s' % target),
        header_filename=lambda: build.find_header('symbolic.h', in_path='include'),
        rtld_flags=['NOW', 'NODELETE']
    )


setup(
    name='symbolic-python',
    version=version,
    packages=find_packages(),
    include_package_data=True,
    zip_safe=False,
    platforms='any',
    install_requires=[
        'milksnake',
    ],
    setup_requires=[
        'milksnake',
    ],
    milksnake_tasks=[
        build_native,
    ]
)
