# Test image library

Images are grouped by the kind of content they represent, because different
content stresses different compressors. Point the runner at any folder (it walks
subdirectories), e.g. `cargo run -p runner -- compare images/screenshots`.

| Folder | Content | What it stresses |
|---|---|---|
| `math/` | 25 generated synthetic patterns (`cargo run -p runner --bin gen`) | Designed per-algorithm — see top-level README |
| `cartoon/` | Flat-color illustrations | Long runs / few colors → RLE, PNG, Huffman |
| `real-life/` | Kodak photo set | Continuous-tone natural images → the hard case |
| `screenshots/` | UI, text, charts *(drop your own)* | Sharp edges + flat regions → lossless codecs |
| `gradients/` | Smooth tonal transitions *(drop your own)* | Low-frequency signal → DCT-style methods |
| `textures/` | High-frequency natural detail *(drop your own)* | Worst case — little redundancy |

The `screenshots/`, `gradients/`, and `textures/` folders ship empty (just a
`.gitkeep`). Drop `.png` (or `.jpg`, `.bmp`, `.webp`, …) files in and the runner
picks them up automatically.

## Where to get free images

All of these are freely downloadable and commonly used for compression work:

- **Kodak True Color set** — http://r0k.us/graphics/kodak/ — 24 classic 768×512 photos (the `real-life/` set).
- **CLIC** (Challenge on Learned Image Compression) — https://clic.compression.cc/ — professional + mobile photo datasets made for codec benchmarking.
- **Tecnick TESTIMAGES** — https://testimages.org/ — high-resolution test images across many categories.
- **USC-SIPI Image Database** — https://sipi.usc.edu/database/ — the classic research set (Lena, Mandrill, aerials, textures).
- **DIV2K** — https://data.vision.ee.ethz.ch/cvl/DIV2K/ — 2K high-quality images (heavy, but excellent variety).
- **xiph.org test media** — https://media.xiph.org/ — large lossless source images/video frames.
- **Unsplash / Pexels / Pixabay** — https://unsplash.com · https://pexels.com · https://pixabay.com — free-license stock photos; good for `textures/` and `gradients/`.
- **Wikimedia Commons** — https://commons.wikimedia.org/ — public-domain / CC media, including high-res scans and screenshots.

For `screenshots/`, your own desktop/app captures are ideal — they're the
flat-color-plus-text case where lossless codecs shine.
