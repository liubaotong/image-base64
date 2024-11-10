use base64::{engine::general_purpose::STANDARD, Engine};
use gloo::file::callbacks::FileReader;
use gloo::file::File;
use std::collections::HashMap;
use web_sys::{Event, HtmlInputElement, HtmlImageElement, MouseEvent};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::{JsCast, closure::Closure};
use yew::prelude::*;

pub enum Msg {
    FileSelected(File),
    Loaded(String),
    Files(Vec<File>),
    ToggleModal,
    CopyBase64,
    ResetCopyButton,
    UpdateDimensions(String),
    UpdateImageInfo(String, String),
}

#[derive(Clone, Default)]
struct ImageInfo {
    format: String,
    size: String,
    dimensions: String,
    mime_type: String,
    aspect_ratio: String,
}

pub struct Model {
    readers: HashMap<String, FileReader>,
    base64_data: Option<String>,
    modal_open: bool,
    copy_button_text: String,
    image_info: Option<ImageInfo>,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            readers: HashMap::default(),
            base64_data: None,
            modal_open: false,
            copy_button_text: "复制".to_string(),
            image_info: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::FileSelected(file) => {
                let file_name = file.name();
                let size = file.size();
                let format = get_file_format(&file_name);
                let mime_type = file.raw_mime_type();
                
                self.image_info = Some(ImageInfo {
                    format,
                    size: format_size(size),
                    dimensions: String::from("加载中..."),
                    mime_type,
                    aspect_ratio: String::from("加载中..."),
                });

                let task = {
                    let link = ctx.link().clone();
                    gloo::file::callbacks::read_as_bytes(&file, move |res| {
                        let bytes = res.expect("failed to read file");
                        let base64 = STANDARD.encode(bytes);
                        link.send_message(Msg::Loaded(base64));
                    })
                };
                self.readers.insert(file_name, task);
                true
            }
            Msg::Loaded(data) => {
                self.base64_data = Some(data.clone());
                if let Some(_) = &mut self.image_info {
                    let img = HtmlImageElement::new().unwrap();
                    let link = ctx.link().clone();
                    let on_load = Closure::wrap(Box::new(move |event: Event| {
                        let img = event.target().unwrap().dyn_into::<HtmlImageElement>().unwrap();
                        let width = img.natural_width();
                        let height = img.natural_height();
                        let dimensions = format!("{}×{} px", width, height);
                        
                        let ratio = calculate_aspect_ratio(width, height);
                        let aspect_ratio = format!("{}:{}", ratio.0, ratio.1);
                        
                        link.send_message(Msg::UpdateImageInfo(dimensions, aspect_ratio));
                    }) as Box<dyn FnMut(Event)>);
                    
                    img.set_onload(Some(on_load.as_ref().unchecked_ref()));
                    on_load.forget();
                    img.set_src(&format!("data:image/png;base64,{}", data));
                }
                true
            }
            Msg::UpdateDimensions(dimensions) => {
                if let Some(info) = &mut self.image_info {
                    info.dimensions = dimensions;
                }
                true
            }
            Msg::Files(files) => {
                for file in files.into_iter() {
                    ctx.link().send_message(Msg::FileSelected(file));
                }
                true
            }
            Msg::ToggleModal => {
                self.modal_open = !self.modal_open;
                true
            }
            Msg::CopyBase64 => {
                if let Some(base64) = &self.base64_data {
                    if let Some(window) = web_sys::window() {
                        let navigator = window.navigator();
                        let base64 = base64.clone();
                        let size = format_size(base64.len() as u64);
                        let clipboard = navigator.clipboard();
                        wasm_bindgen_futures::spawn_local(async move {
                            let promise = clipboard.write_text(&base64);
                            let _ = JsFuture::from(promise).await;
                        });
                        self.copy_button_text = format!("已复制 {}", size);
                        let link = ctx.link().clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            gloo_timers::future::sleep(std::time::Duration::from_millis(2000)).await;
                            link.send_message(Msg::ResetCopyButton);
                        });
                    }
                }
                true
            }
            Msg::ResetCopyButton => {
                self.copy_button_text = "复制".to_string();
                true
            }
            Msg::UpdateImageInfo(dimensions, aspect_ratio) => {
                if let Some(info) = &mut self.image_info {
                    info.dimensions = dimensions;
                    info.aspect_ratio = aspect_ratio;
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_change = ctx.link().callback(|e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                let mut result = Vec::new();
                for i in 0..files.length() {
                    if let Some(file) = files.get(i) {
                        result.push(File::from(file));
                    }
                }
                Msg::Files(result)
            } else {
                Msg::Files(vec![])
            }
        });

        let copy_base64 = ctx.link().callback(|_| Msg::CopyBase64);

        let copy_button_class = if self.copy_button_text.starts_with("已复制") {
            "copy-button copied"
        } else {
            "copy-button"
        };

        let toggle_modal = ctx.link().callback(|_| Msg::ToggleModal);

        html! {
            <div class="container">
                <h1>{ "图片上传生成 Base64" }</h1>
                <div class="image-container">
                    <input type="file" id="file-input" accept="image/*" onchange={on_change} class="file-input" />
                    <label 
                        for="file-input" 
                        class={classes!(
                            "upload-area",
                            self.base64_data.is_some().then_some("has-image")
                        )}
                    >
                        {
                            if let Some(base64) = &self.base64_data {
                                html! {
                                    <>
                                        <img 
                                            src={format!("data:image/png;base64,{}", base64)} 
                                            class="visible" 
                                            alt="Image Preview"
                                        />
                                        <button 
                                            class="preview-button"
                                            onclick={toggle_modal.clone()}
                                        >
                                            { "预览" }
                                        </button>
                                    </>
                                }
                            } else {
                                html! {}
                            }
                        }
                    </label>

                    if let Some(info) = &self.image_info {
                        <div class="image-info">
                            <div class="info-item">
                                <span class="info-label">{ "图片格式" }</span>
                                <span class="info-value">{ &info.format }</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">{ "MIME类型" }</span>
                                <span class="info-value">{ &info.mime_type }</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">{ "文件大小" }</span>
                                <span class="info-value">{ &info.size }</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">{ "图片尺寸" }</span>
                                <span class="info-value">{ &info.dimensions }</span>
                            </div>
                            <div class="info-item">
                                <span class="info-label">{ "纵横比" }</span>
                                <span class="info-value">{ &info.aspect_ratio }</span>
                            </div>
                        </div>
                    }
                </div>

                if self.modal_open {
                    if let Some(base64) = &self.base64_data {
                        <div class="modal-overlay active" onclick={toggle_modal.clone()}>
                            <div class="modal-content" onclick={|e: MouseEvent| e.stop_propagation()}>
                                <button class="modal-close" onclick={toggle_modal}></button>
                                <img 
                                    src={format!("data:image/png;base64,{}", base64)} 
                                    alt="Full size preview" 
                                />
                            </div>
                        </div>
                    }
                }

                if let Some(base64) = &self.base64_data {
                    <div class="base64-output-container">
                        <div class="base64-output">
                            { base64 }
                        </div>
                        <button 
                            class={copy_button_class}
                            onclick={copy_base64}
                        >
                            {
                                if self.copy_button_text.starts_with("已复制") {
                                    let parts: Vec<&str> = self.copy_button_text.splitn(2, ' ').collect();
                                    html! {
                                        <>
                                            <span class="copy-text">{ parts[0] }</span>
                                            <span class="copy-size">{ parts[1] }</span>
                                        </>
                                    }
                                } else {
                                    html! { &self.copy_button_text }
                                }
                            }
                        </button>
                    </div>
                } else {
                    <p class="base64-output">{ "未选择文件" }</p>
                }
            </div>
        }
    }
}

fn format_size(size: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

fn get_file_format(filename: &str) -> String {
    filename
        .split('.')
        .last()
        .map(|s| s.to_uppercase())
        .unwrap_or_else(|| String::from("未知"))
}

fn calculate_aspect_ratio(width: u32, height: u32) -> (u32, u32) {
    let gcd = gcd(width, height);
    (width / gcd, height / gcd)
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

fn main() {
    yew::Renderer::<Model>::new().render();
}
