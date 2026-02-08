{
  lib,
  buildPythonPackage,
  fetchFromGitHub,
  cargo,
  cffi,
  pkg-config,
  rustPlatform,
  rustc,
  setuptools,
  openssl,
  typing-extensions,
  sphinx,
  sphinx-inline-tabs,
  sphinx-rtd-theme,
  pyenchant,
  readme-renderer,
  sphinxcontrib-spelling,
  nox,
  check-sdist,
  click,
  mypy,
  ruff,
  build,
  bcrypt,
  certifi,
  cryptography-vectors,
  pretend,
  pytest,
  pytest-benchmark,
  pytest-cov,
  pytest-xdist,
  pytest-randomly,
}:

buildPythonPackage (finalAttrs: {
  pname = "cryptography";
  version = "46.0.4";
  pyproject = true;

  src = fetchFromGitHub {
    owner = "pyca";
    repo = "cryptography";
    tag = finalAttrs.version;
    hash = "sha256-vT99F/wmd/ipgASmIcQITXNgng69Nn7cN66W2cXOzZY=";
  };

  cargoDeps = rustPlatform.fetchCargoVendor {
    inherit (finalAttrs) pname version src;
    hash = "sha256-5ElDEl7MdcQfu/hy+POSBcvkNCFAMo6La5s6uRhZ/fM=";
  };

  build-system = [
    cargo
    cffi
    pkg-config
    rustPlatform.cargoSetupHook
    rustPlatform.maturinBuildHook
    rustc
    setuptools
  ];

  buildInputs = [
    openssl
  ];

  dependencies = [
    cffi
    typing-extensions
  ];

  optional-dependencies = {
    docs = [
      sphinx
      sphinx-inline-tabs
      sphinx-rtd-theme
    ];
    docstest = [
      pyenchant
      readme-renderer
      sphinxcontrib-spelling
    ];
    nox = [
      nox
    ];
    pep8test = [
      check-sdist
      click
      mypy
      ruff
    ];
    sdist = [
      build
    ];
    ssh = [
      bcrypt
    ];
    test = [
      certifi
      cryptography-vectors
      pretend
      pytest
      pytest-benchmark
      pytest-cov
      pytest-xdist
    ];
    test-randomorder = [
      pytest-randomly
    ];
  };

  pythonImportsCheck = [
    "cryptography"
  ];

  meta = {
    description = "[..]";
    homepage = "https://github.com/pyca/cryptography";
    changelog = "https://github.com/pyca/cryptography/blob/${finalAttrs.src.rev}/CHANGELOG.rst";
    license = with lib.licenses; [
      asl20
      bsd3
    ];
    maintainers = with lib.maintainers; [ alice ];
  };
})
