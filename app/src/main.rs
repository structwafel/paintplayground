use dioxus::prelude::*;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");

fn main() {
    dioxus::launch(App);
}
struct GridState {
    pixels: Vec<u8>, // Store colors as color indices (0-15)
    selected_color: u8,
    translation: (f64, f64),
    scale: f64,
}

#[component]
fn App(cx: Scope) -> Element {
    let grid_state = use_ref(cx, || GridState {
        pixels: vec![0; 100 * 100],
        selected_color: 1,
        translation: (0.0, 0.0),
        scale: 1.0,
    });

    let canvas_ref = use_ref(cx, || None::<HtmlCanvasElement>);

    // Draw function
    let draw_canvas = move |_| {
        if let Some(canvas) = &*canvas_ref.read() {
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap();

            let state = grid_state.read();

            context.save();
            context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
            context.translate(state.translation.0, state.translation.1);
            context.scale(state.scale, state.scale);

            // Draw pixels
            for y in 0..100 {
                for x in 0..100 {
                    let color_index = state.pixels[y * 100 + x];
                    context.set_fill_style(&get_color_from_index(color_index).into());
                    context.fill_rect(x as f64 * 10.0, y as f64 * 10.0, 10.0, 10.0);
                }
            }

            context.restore();
        }
    };

    // Handle canvas click
    let handle_canvas_click = move |evt: MouseEvent| {
        if let Some(canvas) = &*canvas_ref.read() {
            let rect = canvas.get_bounding_client_rect();
            let state = grid_state.read();

            let x = ((evt.client_x() as f64 - rect.left() - state.translation.0)
                / (10.0 * state.scale)) as usize;
            let y = ((evt.client_y() as f64 - rect.top() - state.translation.1)
                / (10.0 * state.scale)) as usize;

            if x < 100 && y < 100 {
                grid_state.write().pixels[y * 100 + x] = state.selected_color;
                draw_canvas(());

                // Send update to server via WebSocket
                send_update_to_server(x, y, state.selected_color);
            }
        }
    };

    // Initialize WebSocket connection
    use_effect(|_| {
        connect_to_server(0, 0);
        async move {}
    });

    cx.render(rsx! {
        div { class: "container",
            h1 { "Dioxus Paint Sandbox" }

            // Color picker
            div { class: "color-picker",
                (0..16).map(|i| {
                    rsx! {
                        button {
                            key: "{i}",
                            class: "color-button",
                            style: "background-color: {get_color_from_index(i)};",
                            onclick: move |_| {
                                grid_state.write().selected_color = i;
                            }
                        }
                    }
                })
            }

            // Canvas for drawing
            canvas {
                width: "1000",
                height: "1000",
                ref: |el| {
                    if let Some(canvas_element) = el.cast::<HtmlCanvasElement>() {
                        *canvas_ref.write() = Some(canvas_element.clone());
                        draw_canvas(());
                    }
                },
                onclick: handle_canvas_click,
                onmousedown: move |evt| { /* Pan/zoom logic */ },
                onmouseup: move |_| { /* End pan/zoom */ },
                onwheel: move |evt| { /* Zoom logic */ }
            }

            // Navigation controls
            div { class: "controls",
                button { onclick: move |_| { /* Move logic */ }, "⬆️" }
                div {
                    button { onclick: move |_| { /* Move logic */ }, "⬅️" }
                    button { onclick: move |_| { /* Move logic */ }, "➡️" }
                }
                button { onclick: move |_| { /* Move logic */ }, "⬇️" }
            }

            p { id: "location", "Current location: (0, 0)" }
        }
    })
}

#[component]
pub fn Hero() -> Element {
    rsx! {
        div {
            id: "hero",
            img { src: HEADER_SVG, id: "header" }
            div { id: "links",
                a { href: "https://dioxuslabs.com/learn/0.6/", "📚 Learn Dioxus" }
                a { href: "https://dioxuslabs.com/awesome", "🚀 Awesome Dioxus" }
                a { href: "https://github.com/dioxus-community/", "📡 Community Libraries" }
                a { href: "https://github.com/DioxusLabs/sdk", "⚙️ Dioxus Development Kit" }
                a { href: "https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus", "💫 VSCode Extension" }
                a { href: "https://discord.gg/XgGxMSkvUM", "👋 Community Discord" }
            }
        }
    }
}
