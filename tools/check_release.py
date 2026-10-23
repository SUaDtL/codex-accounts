"""Check that release versions and notes are synchronized without publishing."""
from publish_release import changelog_notes, manifest_version, release_tag


def main() -> None:
    version = manifest_version()
    release_tag(version)
    changelog_notes(version)
    print(f'Release metadata: {version}')


if __name__ == '__main__':
    main()
