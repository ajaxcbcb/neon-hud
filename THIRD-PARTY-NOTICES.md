# Third-party materials

- OpenAI icon: Simple Icons v13.0.0, `icons/openai.svg`, CC0-1.0. [Source](https://github.com/simple-icons/simple-icons/blob/13.0.0/icons/openai.svg), [license](https://github.com/simple-icons/simple-icons/blob/13.0.0/LICENSE.md). Trademark rights remain with the provider.
- Claude icon: Lobe Icons, `packages/static-svg/icons/claude.svg`, MIT. [Source](https://github.com/lobehub/lobe-icons/blob/master/packages/static-svg/icons/claude.svg), [license](https://github.com/lobehub/lobe-icons/blob/master/LICENSE). Copyright LobeHub. Full applicable license is bundled under `public/brands/LOBE-LICENSE`.
- Original Neon HUD application mark: project license.
- Anime.js 4.5.0: MIT, copyright Julian Garnier. [Project](https://animejs.com/). Full license: `public/licenses/ANIMEJS-MIT.txt`.
- Bricolage Grotesque: SIL Open Font License 1.1, copyright 2022 The Bricolage Grotesque Project Authors. Full license: `public/licenses/BRICOLAGE-OFL.txt`.
- Space Mono: SIL Open Font License 1.1, copyright 2016 The Space Mono Project Authors. Full license: `public/licenses/SPACE-MONO-OFL.txt`.

The original arcade interface draws inspiration from [Uiverse button effects](https://uiverse.io/button-effects). No Uiverse component source is bundled.

The native renderer uses egui/eframe 0.33.3 (MIT OR Apache-2.0), copyright Emil Ernerfeldt. Its default fonts include Hack, Noto Emoji, Ubuntu Light and emoji-icon-font. Their upstream notices and full licenses are copied unchanged from [egui 0.33.3](https://github.com/emilk/egui/tree/0.33.3/crates/epaint_default_fonts/fonts) into `src-native/licenses/` and native preview packages. Native metric symbols are drawn by the application rather than loading the web renderer's graphic files.

Native settings also embed Ubuntu Bold under the Ubuntu Font Licence 1.0. The unchanged font is sourced from [Google Fonts commit a50de978](https://github.com/google/fonts/tree/a50de97857f6626d6628c91e6a34c12ee8a16ede/ufl/ubuntu); its full license and copyright notice are included in `src-native/licenses/`. Font SHA-256: `679b5c1e09cab3156bb8ef529735f9382bf31ca7ac737382ab959297f8d82ad4`.

Dependency versions are recorded in `package-lock.json`, `src-tauri/Cargo.lock` and `src-native/Cargo.lock`. Svelte, Vite and their dependencies retain their upstream licenses; Tauri and Rust dependencies retain theirs. Release packages include project license files and provider graphic notices. This list does not relicense third-party dependencies or trademarks.
