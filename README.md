# Dept. of Forestry

Goal of this demo project is to showcase:
* checkpointing environments
* pushing/pulling environments (at certain checkpoints)

## Dev env

To setup your dev env, create a conda env

```
$ conda env create -f environment.yml 

$ conda activate dof-dev
```

## Try it out

### `dof checkpoint`
This command will checkpoint your environment. That is, take a snapshot of
what is currently installed so that you can go back to it at any time.

```
$ dof checkpoint save
```

To list all the current available checkpoints run

```
$ dof checkpoint list
```

To see all the changes to the environment since your last checkpoint

```
$ dof checkpoint diff --rev <revision uuid>
```

To see all the packages in an environment

```
 $ dof checkpoint show --rev <revision uuid> 
 ```

#### Example

Start with the `dof-dev` environment
```
$ conda env create -f environment.yml 
. . .
$ conda activate dof-dev

# ensure dof is installed
$ python -m pip install -e .
```

Now, try to create some checkpoints and install some packages
```bash
# createa a checkpoint
(dof-dev) sophia:dof/ (main✗) $ dof checkpoint save  
(dof-dev) sophia:dof/ (main✗) $ dof checkpoint list              
                                                 Checkpoints                                                  
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ uuid                             ┃ tags                                 ┃ timestamp                        ┃
┡━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┩
│ 1bc3a3a434454437a9f72061672b8189 │ ['1bc3a3a434454437a9f72061672b8189'] │ 2025-01-25 00:00:07.707080+00:00 │
└──────────────────────────────────┴──────────────────────────────────────┴──────────────────────────────────┘

# install a new package
(dof-dev) sophia:dof/ (main✗) $ conda install jinja2  

# check the diff
(dof-dev) sophia:dof/ (main✗) $ dof checkpoint diff --rev 1bc3a3a434454437a9f72061672b8189            
diff with rev 1bc3a3a434454437a9f72061672b8189
+ url='https://conda.anaconda.org/conda-forge/linux-64/markupsafe-3.0.2-py312h178313f_1.conda'
+ url='https://conda.anaconda.org/conda-forge/noarch/jinja2-3.1.5-pyhd8ed1ab_0.conda'

# save a new checkpoint
(dof-dev) sophia:dof/ (main✗) $ dof checkpoint save 
(dof-dev) sophia:dof/ (main✗) $ dof checkpoint list     
                                                 Checkpoints                                                  
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ uuid                             ┃ tags                                 ┃ timestamp                        ┃
┡━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┩
│ d1256c34d93c48eb96ccfb125c598ffe │ ['d1256c34d93c48eb96ccfb125c598ffe'] │ 2025-01-25 00:02:44.065559+00:00 │
│ 1bc3a3a434454437a9f72061672b8189 │ ['1bc3a3a434454437a9f72061672b8189'] │ 2025-01-25 00:00:07.707080+00:00 │
└──────────────────────────────────┴──────────────────────────────────────┴──────────────────────────────────┘

# try to install a pip package
(dof-dev) sophia:dof/ (main✗) $ python -m pip install blinker

# check the diff from the last checkpoint
(dof-dev) sophia:dof/ (main✗) $ dof checkpoint diff --rev d1256c34d93c48eb96ccfb125c598ffe     
diff with rev d1256c34d93c48eb96ccfb125c598ffe
+ name='blinker' version='1.9.0' build='pypi_0' url=None

# check the diff from the first checkpoint
(dof-dev) sophia:dof/ (main✗) $ dof checkpoint diff --rev 1bc3a3a434454437a9f72061672b8189  
diff with rev 1bc3a3a434454437a9f72061672b8189
+ name='blinker' version='1.9.0' build='pypi_0' url=None
+ url='https://conda.anaconda.org/conda-forge/linux-64/markupsafe-3.0.2-py312h178313f_1.conda'
+ url='https://conda.anaconda.org/conda-forge/noarch/jinja2-3.1.5-pyhd8ed1ab_0.conda'

# remove a dependency
(dof-dev) sophia:dof/ (main✗) $ conda uninstall jinja2

# now recheck the diff from the last checkpoint
(dof-dev) sophia:dof/ (main✗) $ dof checkpoint diff --rev d1256c34d93c48eb96ccfb125c598ffe    
diff with rev d1256c34d93c48eb96ccfb125c598ffe
+ name='blinker' version='1.9.0' build='pypi_0' url=None
- url='https://conda.anaconda.org/conda-forge/linux-64/markupsafe-3.0.2-py312h178313f_1.conda'
- url='https://conda.anaconda.org/conda-forge/noarch/jinja2-3.1.5-pyhd8ed1ab_0.conda'
```

### pushing and pulling to park

#### setup [park server](https://github.com/soapy1/park)

```bash
$ git clone https://github.com/soapy1/park.git
$ cd park
$ pixi install
# run the server on port 8000
$ pixi run dev
```

#### configure dof

Make sure the park server is running and then set the `PARK_URL` environment variable.

```bash
$ export PARK_URL=http://localhost:8000
```

#### push a checkpoint to park

To push a checkpoint to park, you can use the `dof push` command.

```bash
$ dof push --target <namespace>/<environment>:<tag> --rev <revision uuid>
```

#### pull a checkpoint from park

To pull a checkpoint from park, you can use the `dof pull` command.

```bash
$ dof pull --target <namespace>/<environment>:<tag> --rev <revision uuid>
```

#### full example

```bash
$ dof checkpoint save
$ dof checkpoint list                                             
                         Checkpoints                          
┏━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ uuid     ┃ tags         ┃ timestamp                        ┃
┡━━━━━━━━━━╇━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┩
│ 8e45de08 │ ['8e45de08'] │ 2025-02-04 01:39:19.260525+00:00 │
└──────────┴──────────────┴──────────────────────────────────┘


$ dof push -t sophia/dof-dev:8e45de08 --rev 8e45de08

$ curl http://localhost:8000/sophia                                    
{"data":{"namespace":"sophia","environments":["dof-dev"]}}  

$ curl http://localhost:8000/sophia/dof-dev       
{"data":{"namespace":"sophia","environment":"dof-dev","checkpoints":["8e45de08"]}}  

$ curl http://localhost:8000/sophia/dof-dev/8e45de08 | jq  --raw-output .data.checkpoint_data

$ dof checkpoint delete --rev 8e45de08

$ dof checkpoint list
        Checkpoints        
┏━━━━━━┳━━━━━━┳━━━━━━━━━━━┓
┃ uuid ┃ tags ┃ timestamp ┃
┡━━━━━━╇━━━━━━╇━━━━━━━━━━━┩
└──────┴──────┴───────────┘


$ dof pull -t sophia/dof-dev:8e45de08 --rev 8e45de08

$ dof checkpoint list                                               
                         Checkpoints                          
┏━━━━━━━━━━┳━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ uuid     ┃ tags         ┃ timestamp                        ┃
┡━━━━━━━━━━╇━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┩
│ 8e45de08 │ ['8e45de08'] │ 2025-02-04 01:39:19.260525+00:00 │
└──────────┴──────────────┴──────────────────────────────────┘
```
