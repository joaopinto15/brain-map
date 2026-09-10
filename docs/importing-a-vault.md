# Import a vault from a git remote or a drive

A vault that is not on this machine yet goes in the same field, and on the same command
line, as one that is.

```sh
brain-map https://github.com/you/notes   # any git remote: https, ssh, git@host:path
brain-map gdrive:notes                   # any rclone remote: the drive, and a folder on it
```

brain-map fetches it into `$XDG_CACHE_HOME/brain-map/vaults` and opens it from there.
Open it again and only what changed comes down. An offline machine opens the last import
instead of failing.

## Set up a drive

Drives go through [rclone](https://rclone.org), which already holds your accounts and
tokens. Run `rclone config` once, then type the name you gave the remote.

| Drive | `rclone config` type | Then |
|-------|----------------------|------|
| Google Drive | `drive` | `brain-map gdrive:notes` |
| OneDrive | `onedrive` | `brain-map onedrive:vaults/brain` |
| Dropbox, S3, Nextcloud, SFTP, and [about 70 more](https://rclone.org/overview/) | any | `brain-map remote:path` |

brain-map knows none of these services by name. It runs `rclone copy`, so a drive rclone
supports is a drive this supports.

If the drive's own desktop client already syncs the vault into a local folder, open that
folder and skip all of this.

## Notes are copied, never deleted

An import runs `rclone copy`, not `rclone sync`. A note you wrote from the window
survives a drive that no longer has it. The cost is that a note you deleted on the drive
stays in the cache. To clear it out, delete the vault's folder under the cache and import
again.

## Git remotes

The first import clones at depth 1. Every open after that is a fast-forward pull.

A first import that fails leaves nothing behind and tells you what git or rclone said. A
later one that fails prints the complaint and opens the copy already on disk, which is
what makes an offline machine work.
