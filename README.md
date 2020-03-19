# Confit

Making sure your work is properly preserved!

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

## Background

Confit is designed to address a simple but very common problem:
the question of whether one's work has been
properly and completely commited
to version control becomes moderately complicated in Git.
For instance,
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

## License

As you can see in the [LICENSE](./LICENSE) file,
Confit is licensed under the LicenseZero Prosperity license.
In part, this is to help normalize the idea of supporting
public software development.
