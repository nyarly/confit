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
is in the commandline help text:
```
Confit 1.0
Judson Lester <nyarly@gmail.com>
makes sure your work is properly preserved in git

USAGE:
    confit [FLAGS] [OPTIONS]

FLAGS:
        --debug      outputs debug data
    -h, --help       Prints help information
    -q, --quiet      suppress normal state summary; scripts can rely on the status code
    -V, --version    Prints version information

OPTIONS:
    -c <checks>...            [possible values: commit, detached, git_prompt, local, merge, push, push_tag, stage, tag,
                             track_files, track_remote]
    -f, --format <format>    choose a format for output [default: summary]  [possible values: macros, debug, statusline,
                             summary]
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
