{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule (finalAttrs: {
  pname = "fzf";
  version = "0.67.0";

  src = fetchFromGitHub {
    owner = "junegunn";
    repo = "fzf";
    tag = "v${finalAttrs.version}";
    hash = "sha256-P6jyKskc2jT6zMLAMxklN8e/630oWYT4bWim20IMKvo=";
  };

  vendorHash = "sha256-uFXHoseFOxGIGPiWxWfDl339vUv855VHYgSs9rnDyuI=";

  ldflags = [
    "-s"
    "-w"
    "-X=main.version=${finalAttrs.version}"
    "-X=main.revision=${finalAttrs.src.rev}"
  ];

  meta = {
    description = "[..]";
    homepage = "https://github.com/junegunn/fzf";
    changelog = "https://github.com/junegunn/fzf/blob/${finalAttrs.src.rev}/CHANGELOG.md";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ alice ];
    mainProgram = "fzf";
  };
})
