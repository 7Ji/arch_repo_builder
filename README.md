# Arch Repository Builder

A multi-threaded builder to build packages and create a sane folder structure for an Arch repo, written initially for https://github.com/7Ji/archrepo

## Build
Run the following command inside this folder
```
cargo build --release
```
The output binary would be `target/release/arch_repo_builder`

Optionally, strip the binary so it would take less space, and place it to somewhere convenient to run (e.g. ~/bin)
```
strip target/release/arch_repo_builder -o output/path
```

## Usage
```
Usage: arch_repo_builder [OPTIONS] [CONFIG]

Arguments:
  [CONFIG]  Optional config.yaml file [default: config.yaml]

Options:
  -p, --proxy <PROXY>  HTTP proxy to retry for git updating and http(s) netfiles if attempt without proxy failed
  -P, --holdpkg        Hold versions of PKGBUILDs, do not update them
  -G, --holdgit        Hold versions of git sources, do not update them
  -I, --skipint        Skip integrity check for netfile sources if they're found
  -B, --nobuild        Do not actually build the packages
  -C, --noclean        Do not clean unused sources and outdated packages
  -N, --nonet          Disallow any network connection during makepkg's build routine
  -g, --gmr <GMR>      Prefix of a 7Ji/git-mirrorer instance, e.g. git://gmr.lan, The mirror would be tried first before actual git remote
  -s, --sign <SIGN>    The GnuPG key ID used to sign packages
  -h, --help           Print help
  -V, --version        Print version
```

**Note: You must run the builder with sudo as a normal user, the builder would drop back to the normal user you call sudo with. This is for the purpose of unattended chroot deployment, bind-mounting, etc, as it could quickly use setuid and setgid syscalls to return to root. Don't worry, the builder would only run those root stuffs in forked child, not in itself.**

## Config
The `config.yaml` would contain a `pkgbuilds` part with simple lines of `name: url`, e.g.:
```
pkgbuilds:
  ampart: https://aur.archlinux.org/ampart.git/
  chormium-mpp: https://aur.archlinux.org/chromium-mpp.git
  yaopenvfd: https://aur.archlinux.org/yaopenvfd.git
```
Addtionally, most CLI options could be set in the config if they're used very often and not considered optional anymore, e.g.:
```
sign: 8815547B7B80370675B3CD20BA27F219383BB875
proxy: http://xray.lan:1081
gmr: git://gmr.lan
noclean: true
pkgbuilds:
  ...
```
Non-CLI options include:
```
basepkgs: [base-devel, distcc]
dephash_strategy: none
```
These are left out of CLI options as you shouldn't change them often:
 - `basepkgs` defines a list of packages that should be installed into the base chroot.
   - If not set then it defaults to `[base-devel]`, which is the most reasonable minimum package set.
   - You might want to modify this if you're using other things, like `distcc`, that's not part of the `base-devel` group for every PKGBUILD.
   - You might want to set explicit `makepkgs` for certain PKGBUILDs instead of changing this, if only they need such deps.
 - `dephash_strategy` defines the strategy used to calculate the dephash, which, if present, will also be part of the pkgid, which then determines the package rebuilds (see below). It accepts the following values:
   - `strict`: consider both deps and makedeps when calculating the dephash, this will result in the most rebuilds, due to possible fake-positive.
   - `loose`: consider only deps when calculating the dephash, fake-positive is less in this case.
   - `none`(default): consider no dep, leave the dephash as 0, and do not consider it when calculating pkgid. This will result in fake-negative, as updates of underlying packages that should trigger rebuilds cannot be found.

The PKGBUILDs could also be defined with advanced options:
```
pkgbuilds:
  ampart: git://git.lan/PKGBUILDs/ampart.git
  xray:
    url: https://aur.archlinux.org/xray.git
    home_binds:
      - go
  dri2to3-git:
    url: https://aur.archlinux.org/dri2to3-git.git
    deps:
      - git
  wiringPi:
    url: git://gmr.lan/github.com/archlinuxarm/PKGBUILDs.git
    branch: master
    subtree: alarm/wiringpi
```
The following optional attributes could be set for each PKGBUILD:
  - `deps`: Explicit dependencies for the package, this is useful if the package maintainer missed such deps. Such packages will also be included when calculating the dep hash. Note this won't be reflected on the result package's metadata, if that's what you want, modify PKGBUILD itself.
  - `makedeps`: Explicit make dependencies for the package, this is useful if the package maintainer missed such deps, e.g. aur/dri2to3-git. Specially, the builder would automatically append `git` to `-git` packages, so you shouldn't need it even if the maintainer missed that. Not included for dephash, not reflected in the result package's metadata, modify PKGBUILD itself if you want that.
  - The above two deps are **appended** to the existing deps defined in PKGBUILD, not replacing them.
  - `branch`: Alternative branch that PKGBUILD should be obtained from. The default is `master`
  - `subtree`: The subtree PKGBUILD should be obtained from, and the whole build folder should be populated via checking out from.
  - `home_binds`: Bind such folders under home into the building chroot, if they exist. The builder would automatically append `go` for packages that depend on `go`, and `.cargo` for packages that depened on `rust/cargo`.

Addtionally, the following aliases are supported for URLs:
  - `AUR` => `format!("https://aur.archlinux.org/{}.git", name)`
    - e.g. `ampart: AUR` would expand to `ampart: https://aur.archlinux.org/ampart.git`
  - `GITHUB/*/` => `format!("https://github.com/{}{}.git", &url[7..], name)`
    - e.g. `yaopenvfd: GITHUB/7Ji-PKGBUILDs/` would expand to `yaopenvfd: https://github.com/7Ji-PKGBUILDs/yaopenvfd.git`
  - `GITHUB/*` => `format!("https://github.com/{}.git", &url[7..])`
    - e.g. `chromium: GITHUB/archlinuxarm/PKGBUILDs` would expand to `chromium: https://github.com/archlinuxarm/PKGBUILDs.git`

## TODO
 - [ ] Resolve inter-dependencies if necessary, to trigger builds if some of our pacakges changed which are deps of other pacakges
   - doing this would also mean splitting builds into multiple steps (build -> install -> build)
 - [ ] Remove all explicit panics introduced in early prototype stage

## Internal
The builder does the following to save a great chunk of build time and resource:
 1. All PKGBUILDs are maintained locally as bare git repos under `sources/PKGBUILDs`, and these repos are updated multi-threadedly, 1 thread per domain (I don't want to put too much load on AUR server)
 2. All git sources are cached locally under `sources/git`, and the update process could take 4 thread per domain.
 3. All network file sources, as long as they have integrity checksums, are cached locally under `sources/file-[integ name]`, and the download process could take 4 thread per domain. And if a file source has multiple checksums, it would only be downloaded once, all remaining cache files are just hard-linked from the first one.
 4. The git and netfile cacher run simultaneously
 5. Build folders `build/[package]` are only populated (also multi-threaded) if either:
    1. The corresponding package has a `pkgver()` function which could only be run after complete source extraction
    2. The corresponding pkgdir `pkg/[pkgid]` is missing, in which `[pkgid]` is generated with `[name]-[commit]-[dephash](-[pkgver])`
 6. Build folder is populated via lightweight checkout (no `.git`) from the local PKGBUILDs bare repos, and symlinks of cached sources. Only vcs sources not with git protocol and netfile sources that do not have integrity checks need to be downloaded for each build.
 7. Packages are stored under `pkg/[pkgid]`. Two folders, `pkg/updated` and `pkg/latest` are created with symlinks, `updated` containing links to packages built during the current run, and `latest` containing links to all latest packages.
    1. `updated` is useful when partial update is wanted
    2. `latest` is useful when full update is wanted
 8. Package dependencies are tracked and solved using native libalpm, and needed deps are cached on host after all PKGBUILDs parsed and a deduplicated dep list is obtained.
 9. Every PKGBUILD is built in its own chroot environment, which is mounted using overlay, with a common minimum base chroot with only `base-devel` installed. The dependencies are all cached on host and are only installed into the overlay chroot when the corresponding package needs building.
### Git source
It might seem redundant that PKGBUILDs and git sources are maintained seperately as bare git repos, although they're both just bare git repos. But the internal logic do treat them differently, as:
  - The PKGBUILDs's bare git repos only track `refs/heads/master` (master branch), and the repos are just stored as `sources/PKGBUILD/[pkgname]`. This means they're both lightweight, taking as little space as possible, and easy for humans to lookup. As we're not storing their work directories, the latter is important when you need to dig the PKGBUILD history and other stuffs.
  - The 'normal' git sources, i.e. those listed in `sources(_[arch])` array in all PKGBUILDs, track both `refs/heads/*` (all branches) and `refs/tags/*` (all tags), but not all `refs/*`. They're stored as `sources/git/[url hash]`. They're more lightweight than those maintained by `makepkg` as the mirror repos it maintain track all `refs/*`. As makepkg could only use branch/tag/commit, the other refs like `refs/pulls/*` (mostly from github repos), `refs/remotes/*`, etc, are meaningless and are killer for our disk space.

### Network file source
We maintain a series of different folders `sources/file-[integ]` to store network file sources that have integrity checksums defined. They're populated after all PKGBUILDs parsed and we got a de-duplicated list of all sources. That means:
  - For future build, network file sources do not need to be re-downloaded, and they can just be symlinked from `sources/file-[integ]`.
  - For any netfile sources, if they're implicity shared between multiple pacakges, as long as they have the same integrity checksum, even with different URLs, they're only downloaded once.
  - For one netfile source, if it has multiple integrity checksums, it would only need to be downloaded once, as long as the other integrity checksums passed the remaining alternatives are just hard-linked.
  - This automatically avoids the case where upstream PKGBUILD maintainer updates a source but kept the file name. Because network files are not tracked by their name nor URL, but only their integrity checksums.

### No network build
There're some bad-behaving packages that acessses the network during their `build()` function, which adds break points to `build()` that not even should be there. This also violates our designing principle that download, extraction and building should happen each in their seperate stages.

The argument `--nonet` could be set to catch such packages, it is achieved by two recursive `unshare` calls, one to unshare the host network namespace while mapping the host non-root user into the container root so a new lo interface could be set up, another recursive one to map the container root back to the host non-root user. 

### Git-mirrorer
It is possible to set the builder to fetch from a [7Ji/git-mirrorer](https://github.com/7Ji/git-mirrorer) instance hosted in local LAN before the actual remote. This can further save the bandwidth usage. And it is highly recommended that you set this up if you're building a lot. Do note that PKGBUILDs won't be fetched from git-mirrorer, as the main source is considered to be AUR, and no PKGBUILD repo should be huge enough to be a problem to update frequently from AUR.

### Chroot
The builder utilizes `chroot()` syscall to run building in dedicated chroots, each package having its own chroot mounted using overlay. There is also an addtional base chroot, which is always populated before even calculating the pkgids, the base chroot serves the addtional purpose that clean repo DBs could be looked up instead of from root, and without breaking the host dependency.