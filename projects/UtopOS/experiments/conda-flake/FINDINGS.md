# Experiment: Nix Flakes Wrapping Conda

## Question
Can we use Nix to pin micromamba itself, then use micromamba to pin Python packages?

## Answer: Yes — with caveats

### What Works

1. **Nix pins the toolchain** — `micromamba` is available in nixpkgs, so Nix can provide a reproducible version of the conda package manager itself.

2. **micromamba pins the packages** — Once you have micromamba, it can install any conda package including CUDA, MKL, and other packages that are difficult to get from nixpkgs.

3. **`nix develop` gives you everything** — A single `nix develop` command gives you a shell with micromamba ready to go, which auto-provisions the conda environment.

4. **The environment.yml is a Nix store path** — The environment definition is hashed and stored in /nix/store, so the _definition_ of your environment is fully reproducible, even if the _packages_ aren't (since conda may resolve differently over time).

### What's Tricky

1. **Network dependency** — `micromamba env create` needs to download packages. This means `nix develop` isn't fully hermetic — it still needs network access on first run. However, the toolchain (micromamba) IS fully reproducible.

2. **conda-lock solves the reproducibility gap** — By using conda-lock to generate a lockfile, and then having Nix provision from that lockfile, you get full reproducibility. The lockfile itself can be committed to git alongside the flake.

3. **Activation in Nix** — conda's `activate` script modifies shell environment variables. In Nix's `mkShell`, you can set these up in `shellHook`, but it's not as clean as pure Nix.

4. **Binary caching** — Nix's binary cache (/nix/store) and conda's binary cache (anaconda/conda-forge channels) are separate. You don't get Nix's caching benefits for conda packages.

### Architecture: The Two-Layer Approach

```
Layer 1: Nix (reproducible tooling)
  ├── micromamba binary
  ├── system libraries
  ├── conda-lock binary
  └── environment.yml definition

Layer 2: conda (reproducible packages)
  ├── Python runtime
  ├── numpy/scipy/pandas (with optimized BLAS)
  ├── CUDA toolkit
  └── pip packages
```

**Key insight**: Nix is better at managing _tools and infrastructure_, while conda is better at managing _scientific Python packages_. They complement each other.

### When to Use This Hybrid

| Scenario | Use Pure Nix | Use Nix+Conda |
|----------|:---:|:---:|
| Web server | ✅ | ❌ |
| CLI tools | ✅ | ❌ |
| ML training (GPU) | ❌ | ✅ |
| Data science notebook | ❌ | ✅ |
| Python package with CUDA deps | ❌ | ✅ |
| Reproducible CI/CD | ✅ | with conda-lock |
| Dev environment for team | ✅ | ✅ |

### Recommended Workflow

1. **Use `flake.nix`** to define your development environment
2. **Use `environment.yml`** to define Python packages
3. **Use `conda-lock`** to lock package versions
4. **Commit all three** to your repo
5. **New team member runs `nix develop`** — they get micromamba + auto-provisioned conda env

### Findings Log

- **micromamba in nixpkgs**: ✅ Available and well-maintained
- **conda-lock in nixpkgs**: ✅ Available
- **Auto-provisioning via shellHook**: ✅ Works well
- **Building a derivation from conda env**: ⚠️ Possible but tricky (the conda env isn't in /nix/store)
- **Full hermetic builds**: ❌ Not without conda-lock + offline channels
- **NixOS module integration**: ✅ Works (see conda-module.nix)

### Next Steps

1. Add conda-lock to the flake for fully locked provisioning
2. Explore building Nix derivations that wrap conda environments
3. Test on actual ML workloads (torch + CUDA)
4. Benchmark: Nix+conda vs pure Nix vs pure conda build times
