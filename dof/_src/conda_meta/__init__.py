# This module provides an interface for reading from the 
# conda-meta directory of an environment. Tools like conda
# and pixi use conda-meta to keep important metadata about
# the environment and it's history.


# interacts with pixi and conda specific
# details in order to understand what the specs requested
# from the user have been. We'll call these requested_specs.
# These are different from dependency_specs which are specs
# that are installed because they are dependencies of the
# requested_specs (and not because the user specifically asked 
# for them).
# There is a case for refactoring this into a pluggable or hook
# based setup. For the purpose of exploring this approach we 
# won't set that up here.
