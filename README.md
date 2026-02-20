# Discord Music Bot

Rust로 작성된 Discord 음악 봇. YouTube 검색/URL 재생, 큐 관리, 반복/셔플 등을 슬래시 커맨드로 제공합니다.

## Tech Stack

- **Rust** (Edition 2021)
- **poise** 0.6 — 슬래시 커맨드 프레임워크
- **serenity** 0.12 — Discord API
- **songbird** 0.4 — 음성 채널 오디오 재생
- **yt-dlp** — YouTube 검색 및 오디오 소스 추출

## 기능

- YouTube URL 또는 검색어로 음악 재생
- 서버별 독립 재생 큐
- 반복 모드 (끔 / 한 곡 / 전체)
- 셔플, 볼륨 조절 (0-100%)
- 일시정지 / 재개
- 음성 채널에 혼자 남으면 30초 후 자동 퇴장

## 슬래시 커맨드

| 커맨드 | 단축 | 설명 |
|--------|------|------|
| `/play <검색어\|URL>` | `/p` | 음악 재생 또는 큐에 추가 |
| `/skip` | `/s` | 현재 곡 건너뛰기 |
| `/stop` | `/st` | 재생 중지 및 퇴장 |
| `/queue [페이지]` | `/q` | 재생 목록 표시 |
| `/pause` | `/pa` | 일시정지 |
| `/resume` | `/r` | 재개 |
| `/nowplaying` | `/np` | 현재 재생 중인 곡 정보 |
| `/loop <off\|song\|queue>` | `/l` | 반복 모드 설정 |
| `/shuffle` | `/sh` | 큐 셔플 |
| `/remove <번호>` | `/rm` | 큐에서 곡 제거 |
| `/volume <0-100>` | `/v` | 볼륨 조절 |

## 시작하기

### 사전 요구사항

- Rust 1.88+
- yt-dlp
- ffmpeg
- [Discord Bot Token](https://discord.com/developers/applications)

### 환경 변수

`.env` 파일을 프로젝트 루트에 생성:

```
DISCORD_TOKEN=<봇 토큰>
```

### 로컬 실행

```bash
cargo build --release
cargo run --release
```

### Docker 실행

```bash
docker compose up --build -d
```

Docker 이미지는 멀티스테이지 빌드를 사용하며, 런타임에 ffmpeg과 yt-dlp를 자동 설치합니다.

## 프로젝트 구조

```
src/
├── main.rs              # 엔트리포인트
├── lib.rs               # 라이브러리 크레이트
├── config.rs            # 환경변수 로드
├── commands/            # 슬래시 커맨드 (11개 + 11 단축 = 22개)
│   ├── play.rs
│   ├── skip.rs
│   ├── stop.rs
│   ├── queue.rs
│   ├── pause.rs
│   ├── resume.rs
│   ├── nowplaying.rs
│   ├── loop_cmd.rs
│   ├── shuffle.rs
│   ├── remove.rs
│   └── volume.rs
├── music/               # 음악 엔진
│   ├── queue.rs         # 서버별 큐 관리
│   ├── player.rs        # 오디오 재생 및 트랙 이벤트
│   └── source.rs        # yt-dlp 연동
├── events/              # 이벤트 핸들러
│   └── voice_state.rs   # 자동 퇴장 로직
└── utils/
    └── embed.rs         # Discord Embed 생성
```

## 테스트

```bash
# 유닛 테스트
cargo test

# 통합 테스트 (yt-dlp 필요)
cargo test -- --ignored

# 린트
cargo fmt --check
cargo clippy -- -D warnings
```

## CI/CD

- **CI**: 모든 PR 및 main push 시 자동 실행 (fmt, clippy, test)
- **Deploy**: main push 시 SSH로 서버에 자동 배포 (`docker compose up --build -d`)
