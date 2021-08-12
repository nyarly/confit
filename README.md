# Confit

Making sure your work is properly preserved (in git)!

![Confit](./jar.svg)

For example:
```bash
⮀ confit
               all files tracked: true
             no unstaged changes: false
           no uncommited changes: true
     commit tracked by local ref: true
            branch tracks remote: true
  all commits merged from remote: true
    all commits pushed to remote: true
        current commit is tagged: false
                   tag is pushed: false
⮀ echo $?
18
```

The error code is computed by bitwise OR
of various "families" of commit checking.

A quick summary of other features
is in the command-line help text:
```
Confit 1.1.1
Judson <nyarly@gmail.com>

Generates reports about the state of version control for the current workspace.
Git status is collected, and then a series of checks are run on it to establish
that the contents of the workspace are stored, synchronized and recoverable.
The results of these checks are then formatted into a report via a selectable
template.

These reports help confirm that you've properly committed, pushed and tagged
your work. This can help smooth collaboration with other humans, as well as
reduce the problem surface when debugging automated tools, like continuous
integration systems.

USAGE:
    confit [FLAGS] [OPTIONS]

FLAGS:
        --debug      outputs debug data
    -h, --help       Prints help information
    -q, --quiet      suppress normal state summary; scripts can rely on the status code
    -V, --version    Prints version information

OPTIONS:
    -c, --checks <checks>...     [possible values: commit, detached,
        git_prompt, local, merge, push, push_tag, stage, tag, track_files, track_remote]
    -f, --format <format>       choose a format for output [default: summary]
        [possible values: summary, statusline, debug]

EXAMPLES

> confit --checks git_prompt --format statusline
main|+3?2

In a fish_prompt.fish:

  set -l statusline (confit -c git_prompt -f statusline)
  test $status -lt 128; and echo -n "⭠ "$statusline

Two of the options to --checks are special: they select groups of checks:
'git_prompt' (suitable for a command line prompt function) and 'local', which
includes only those checks that don't require data collection from the git
remote, which can be useful e.g. to avoid authenticating, or network delays.
The checks performed on the workspace determine what data needs to be
collected. You can select which checks to perform with the --checks flag.

To aid machine use of this tool, its exit status is significant.

Anything over 127 indicates errors running git (for instance: not in a git
workspace), or rendering templates.

Statuses less than or equal to 127 are the bitwise OR of the "status group" of
any failing checks. Those groups are:

   2: Local files uncommitted (unknown, only staged, etc.)
   4: Commits unrecorded to the remote
   8: Remote commits not pulled
  16: Commit not tagged, or tag not pushed
```

## Background

Confit is designed to address a simple but very common problem:
the question of whether one's work has been
properly and completely commited
to version control becomes moderately complicated in Git.
This is enough of a problem that
people make jokes about
pushing your work before leaving the building
in the event of a fire.
More seriously,
reproduceable continuous deployment
relies on being able to recover
the particular state of code
that is represented by a deployed artifact.

Over time,
many ad hoc solutions
have been produced to
address these needs,
variously parsing different aspects of Git output,
in order to print command line prompts,
or manage releases,
or prepare for code generation,
etc, etc.

To my knowledge,
no single tool answers the question
"is this code complete and properly saved?"
Thus, Confit was inspired.

## Details

Confit runs `git` to establish the state of the current workspace.
It uses a Nom parser
to quickly interpret
the results,
and then templates out a report of
nine criteria it uses to define
a "well preserved" workspace.

Not all use cases require all the criteria;
two of them, notably,
require network access to check.
Therefore, `confit` has a flag
to select which checks to run,
as well as two "group" tags:
`local` which bundles checks that don't require network access,
and
`git_prompt` which also excludes the tagging related checks.

Further,
`confit` has flags to select formatting;
most notably, there is
`summary` (which is the default format)
and
`statusline` which is suitable for use
in shell prompts.

## Licensing

This package is
licensed under the Indie Code Catalog
[Free License,](https://indiecc.com/free/2.0.0)
with commercial use available
[for purchase.](https://indiecc.com/~nyarly/confit)
