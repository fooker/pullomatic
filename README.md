`pullomatic` automates GIT repository synchronisation.

Storing configuration or other application data in a GIT repository is a common practice.
Usually custom scripts are used to pull updates from remote from time to time.
`pullomatic` replaces these scripts with pure configuration.

Beside the polling interval, `pullomatic` provides a HTTP endpoint which can be used to trigger updates by web-hooks.

Whenever a change is detected in the remote repository branch, the new branch head will be checked out to the path.

## Configuration

Each repository is configured in a single file which must be placed inside `/etc/pullomatic/`.
The filename is used as repository name and must be formatted as TOML file.

The main section must contain a `path` which specifies the path where the repository lives.
On startup, the existence of the repository will checked.
If the repository does not exists, the remote repository will be cloned to that path.
Second, the config must contain a `remote_url` and a `remote_branch` which specifies the remote URL of the GIT repository and the branch to check out.

The following options are allowed in the configuration:

| Option | Type | Required | Description |
| ------ | ---- | -------- |----------- |
| `path` | `str` | ✓ | Path to the GIT repository on disk |
| `remote_url` | `str` | ✓ | Remote URL of the GIT repository to pull changes from |
| `remote_branch` | `str` | ✓ | The branch to check out and pull changes from |
