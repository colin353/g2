## g2, an alternative terminal interface for git

![crates.io](https://img.shields.io/crates/v/g2.svg)

### Installation

**Using cargo**: Run `cargo install g2`

**From source**: Clone this repository and run `cargo install --path=.`

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

### Usage

Fist, clone a repository using `g2 clone`, e.g.

```
g2 clone git@github.com:colin353/g2.git
```

You can use SSH or HTTPS, whatever works with git works with g2. Note, this checks out the repo
to ~/.g2/repos, not to whatever directory you're in. To start developing, create a branch:

```
g2 new my-bugfix
```

This will create a git worktree branch called `my-bugfix` which is based on `main`. If you have teleport set up,
it will automatically jump you to that directory.

Now make some changes in the branch. If you want to see your changes, you can use

```
g2 status
```

which will show something like 

```
Local branch (my-bugfix)
  [+16, -6] README.md
      [new] my-new-file.txt
```

Here, I've made a couple of changes to my README and added a new file. Next I want to create a PR
based on these changes, so run `g2 upload`.


### Todo list:
 
 - [x] Detect and show merge conflicts better
 - [ ] Bypass gh and create PRs via API?
 - [ ] More info on installation/usage (including `g2 auto`, teleport setup)
 - [ ] Demo GIF
 - [ ] When pushing a PR, change the last commit message to be the PR title?
 - [ ] Reduce dependencies?
 - [ ] Zsh/bash completions
 - [ ] Support branch prefixes
 - [ ] Write docs on usage
