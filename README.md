## g2, an alternative terminal interface for git

### Installation

**With cargo**: Run `cargo install g2`
**From source**: Clone this repository and run `cargo install --path.`

To enable **teleport**, which allows `g2` to change your
current directory, install this into your `~/.zshrc`:

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
 
 - [ ] Publish crate
 - [ ] Bypass gh and create PRs via API?
 - [ ] More info on installation/usage (including `g2 auto`, teleport setup)
 - [ ] Demo GIF
 - [ ] When pushing a PR, change the last commit message to be the PR title?
 - [ ] Reduce dependencies?
 - [ ] Zsh/bash completions
