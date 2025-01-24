# Dept. of Forestry

Goal of this demo project is to showcase:
* using rattler to build conda environments across different platforms
* checkpointing environments
* pushing/pulling environments (at certain checkpoints)

## Dev env

To setup your dev env, create a conda env

```
$ conda env create -f environment.yml 

$ conda activate dof-dev
```

## Try it out

### `dof lock`
This will generate a lockfile for a given environment file

```
$ dof lock --env-file demo-assets/env1.yml
```

### `dof checkpoint`
This command will checkpoint your environment. That is, take a snapshot of
what is currently installed so that you can go back to it at any time.

```
$ dof checkpoint
```

To list all the current available checkpoints run

```
$ dof checkpoint list
```

