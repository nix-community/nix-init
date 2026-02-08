{
  lib,
  python3,
  fetchFromGitHub,
}:

python3.pkgs.buildPythonApplication (finalAttrs: {
  pname = "black";
  version = "26.1.0";
  pyproject = true;

  src = fetchFromGitHub {
    owner = "psf";
    repo = "black";
    tag = finalAttrs.version;
    hash = "sha256-v0XhEPQ8QSXNU9vf8N+xMl/ReoZF1DvEtYZG/g0rngQ=";
  };

  build-system = [
    python3.pkgs.hatch-fancy-pypi-readme
    python3.pkgs.hatch-vcs
    python3.pkgs.hatchling
  ];

  dependencies = with python3.pkgs; [
    click
    mypy-extensions
    packaging
    pathspec
    platformdirs
    pytokens
    tomli
    typing-extensions
  ];

  optional-dependencies = with python3.pkgs; {
    colorama = [
      colorama
    ];
    d = [
      aiohttp
    ];
    jupyter = [
      ipython
      tokenize-rt
    ];
    uvloop = [
      uvloop
    ];
  };

  pythonImportsCheck = [
    "black"
  ];

  meta = {
    description = "[..]";
    homepage = "https://github.com/psf/black";
    changelog = "https://github.com/psf/black/blob/${finalAttrs.src.rev}/CHANGES.md";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "black";
  };
})
