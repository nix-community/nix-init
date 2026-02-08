{
  lib,
  buildPythonPackage,
  fetchFromGitHub,
  flit-core,
  markupsafe,
  babel,
}:

buildPythonPackage (finalAttrs: {
  pname = "jinja";
  version = "3.1.6";
  pyproject = true;

  src = fetchFromGitHub {
    owner = "pallets";
    repo = "jinja";
    tag = finalAttrs.version;
    hash = "sha256-m9ry3cl2IyUUFgZ+gKyw26YJijMY6IM7Gcx6FlQqugU=";
  };

  build-system = [
    flit-core
  ];

  dependencies = [
    markupsafe
  ];

  optional-dependencies = {
    i18n = [
      babel
    ];
  };

  pythonImportsCheck = [
    "jinja2"
  ];

  meta = {
    description = "[..]";
    homepage = "https://github.com/pallets/jinja";
    changelog = "https://github.com/pallets/jinja/blob/${finalAttrs.src.rev}/CHANGES.rst";
    license = lib.licenses.bsd3;
    maintainers = with lib.maintainers; [ alice ];
  };
})
