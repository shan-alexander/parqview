---
tags: [reference, egui_extras, eframe, rust, gui, tablebuilder]
node_type: reference
aliases: [egui-extras-eframe, tablebuilder-eframe-ref]
---
# egui_extras and eframe Crate Reference

Detailed reference for **`eframe`** (the native framework engine for `egui`) and **`egui_extras`** (the extended UI component library providing high-performance virtualized tables).

## 1. eframe (egui Native Framework)

`eframe` provides the native desktop windowing and graphics engine bindings for `egui` applications.

### Core Components & API
- **`eframe::App` Trait**: Primary interface implemented by the application struct.
  - `fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame)`: Called on every frame pass for UI rendering and event processing.
- **`eframe::NativeOptions`**: Configures window dimensions, resizability, transparency, graphics API preference (Glow/OpenGL vs. wgpu), and Wayland/X11 options.
- **`eframe::run_native`**: Entrypoint function to launch the event loop and initialize the GPU context.
- **`eframe::CreationContext`**: Provides initialization context (e.g., accessing storage, customizing egui fonts or style before window display).

### Official Resources
- **Documentation**: https://docs.rs/eframe/latest/eframe/
- **Examples**: https://github.com/emilk/egui/tree/main/crates/eframe

---

## 2. egui_extras (High-Performance Tables & Extensions)

`egui_extras` extends core `egui` with advanced components required for data-dense applications.

### TableBuilder (Data Grid Virtualization)
`egui_extras::TableBuilder` is essential for `parqview` to display large Parquet and DuckDB query results without UI lag.

- **Row Virtualization**: `TableBuilder` calculates scroll bounds and only evaluates / renders rows currently visible in the viewport.
- **Column Sizing Options**:
  - `Column::auto()`: Size based on header or content.
  - `Column::initial(px)`: Initial pixel width with manual resize handle.
  - `Column::remainder()`: Fills remaining window width.
- **Key API Flow**:
  ```rust
  TableBuilder::new(ui)
      .striped(true)
      .resizable(true)
      .column(Column::initial(150.0).resizable(true))
      .column(Column::remainder())
      .header(20.0, |mut header| {
          header.col(|ui| { ui.heading("Column 1"); });
          header.col(|ui| { ui.heading("Column 2"); });
      })
      .body(|body| {
          body.rows(row_height, total_rows, |row_index, mut row| {
              row.col(|ui| { ui.label(get_cell(row_index, 0)); });
              row.col(|ui| { ui.label(get_cell(row_index, 1)); });
          });
      });
  ```

### Official Resources
- **Documentation**: https://docs.rs/egui_extras/latest/egui_extras/
- **TableBuilder Docs**: https://docs.rs/egui_extras/latest/egui_extras/struct.TableBuilder.html
