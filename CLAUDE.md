# Discord Music Bot - Project Rules

## Project Overview

- **Name**: discord-music-bot (every-discord-bot)
- **Tech Stack**: Rust (poise 0.6, serenity 0.12, songbird 0.4)
- **Package Manager**: cargo
- **Deployment**: Docker (multi-stage build)
- **Language**: Korean (한국어 응답)

## Git Rules

### main 브랜치 직접 커밋 금지 (CRITICAL)

- **절대 금지**: `main` 브랜치에 직접 commit/push
- **모든 변경**: `feat/*`, `fix/*`, `hotfix/*` 브랜치 → PR → main 병합
- PR 없이 main에 직접 푸시하는 것은 어떤 경우에도 허용하지 않음

```bash
# 올바른 워크플로우
git checkout main && git pull origin main
git checkout -b feat/my-feature
# ... 작업 ...
git commit -m "feat: add my feature"
git push origin feat/my-feature
# → PR 생성: feat/my-feature → main
```

### Commit Convention

- Format: `<type>: <subject>` (최대 50자, 마침표 없음)
- Types: `init`, `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`, `ci`

## Build & Test

```bash
# 로컬 빌드
cargo build --release

# 테스트 (41개)
cargo test

# 통합 테스트 (yt-dlp 필요)
cargo test -- --ignored

# 린트
cargo fmt --check
cargo clippy -- -D warnings
```

## Project Structure

```
src/
├── main.rs          # 엔트리포인트
├── lib.rs           # 라이브러리 크레이트 (테스트용)
├── config.rs        # 환경변수 로드
├── commands/        # 슬래시 커맨드 (11개 + 11 별칭 = 22개)
├── music/           # 음악 엔진 (큐, 재생, yt-dlp)
├── events/          # 이벤트 핸들러 (자동 퇴장)
└── utils/           # 유틸리티 (임베드)
tests/
├── command_flow.rs        # 커맨드 흐름 통합 테스트
├── command_registration.rs # 커맨드 등록 검증
├── config_test.rs         # 설정 검증
└── yt_dlp_integration.rs  # yt-dlp 연동 테스트 (#[ignore])
```

## Environment Variables

```
DISCORD_TOKEN=<봇 토큰>
APPLICATION_ID=<클라이언트 ID>
```

## CI/CD

- GitHub Actions: push to main, 모든 PR에서 자동 실행
- Jobs: check (fmt+clippy), test (cargo test), integration (yt-dlp 필요)
