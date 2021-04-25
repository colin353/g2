## g2, an alternative terminal interface for git

![crates.io](https://img.shields.io/crates/v/g2.svg)

### Installation

_Using cargo_: Run `cargo install g2`
_From source_: Clone this repository and run `cargo install --path=.`

Once you've installed `g2`, run `g2 check` to verify that your system is set up correctly.

To enable `teleport`, which allows `g2` to change your
current directory in zsh install this into your `~/.zshrc`:

```
g2 () {
  G2=`whence -p g2`
  $G2 $@

  if [ $? -eq 3 ]
  then
    cd `cat /tmp/g2-destination`
  fi
}
g2 auto
```

### Todo list:
 
 - [x] Detect and show merge conflicts better
 - [ ] Bypass gh and create PRs via API?
 - [ ] More info on installation/usage (including `g2 auto`, teleport setup)
 - [ ] Demo GIF
 - [ ] When pushing a PR, change the last commit message to be the PR title?
 - [ ] Reduce dependencies?
 - [ ] Zsh/bash completions
 - [ ] Support branch prefixes
