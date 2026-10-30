<!--
Release notes template. CI (build.yml) drafts a release named "jman vX.Y.Z" when a v* tag is pushed;
paste this into the draft, fill it in, delete these comments and any section that doesn't apply, then publish.
Find what changed with: git log --oneline vPREVIOUS..vX.Y.Z
-->
# jman vX.Y.Z

<!-- One or two sentences: what this release is about, in the user's terms. Bold the main point. -->

## What's new
<!-- User-facing changes, most important first. Say what it means for the user, not how it's implemented. -->
- **Feature:** what it does and why it's useful

## Fixes
<!-- Bugs fixed. Describe the symptom the user saw. -->
- Description of the symptom that's now fixed

## Download
| Platform | File |
| --- | --- |
| Windows | `.exe` (recommended) or `.msi` |
| macOS (Apple Silicon) | `.dmg` |
| Linux | `.AppImage`, `.deb` (Debian/Ubuntu) or `.rpm` (Fedora/openSUSE) |

Intel Macs are not supported in this release.

## Upgrading
<!-- From the previous version. Say whether "Set up" has to be run again (only if setup/env changed). -->
Install vX.Y.Z over your current version. You don't need to run **Set up** again.

## First run
1. Open jman and click **Set up**. On Windows this asks for administrator rights **once**; macOS and Linux don't need them.
2. Open a new terminal and restart your IDE **one last time**. From now on, switching is instant.
3. In your IDE, set the JDK to `~/.jman/current` instead of a specific JDK.

> IntelliJ remembers the version it detected for each SDK, and Gradle/Maven daemons keep the JDK they started with. After switching to a different major version, reload the SDK and run `gradle --stop`.

## Unsigned builds
The installers aren't code-signed yet, so your OS will warn you:
- **Windows:** SmartScreen says "Windows protected your PC". Click **More info → Run anyway**.
- **macOS:** if it says jman "is damaged" or "can't be opened", run `xattr -cr /Applications/jman.app`, or right-click the app and choose **Open**.

## Known issues
- Windows has had the most testing. Please [open an issue](https://github.com/DxxxxY/jman/issues) if something doesn't work on macOS or Linux.
