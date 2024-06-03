mod languages;
mod tokenizer;
mod url;
mod wav;

pub use languages::Languages;
use url::UrlTTS;
pub use wav::mp3_to_wav;

pub const GOOGLE_TTS_MAX_CHARS: usize = 100;

pub struct TTS {
    /// The language of the gTTS client (ISO code)
    ///
    /// example: Languages::English, Languages::Japanese
    language: Languages,
    /// top-level domain of the gTTS client
    ///
    /// example: "com"
    tld: &'static str,
}

impl TTS {
    /// Creates a new gTTS client with the given volume and language.
    pub fn new(language: Languages, tld: Option<&'static str>) -> Self {
        TTS {
            language,
            tld: tld.unwrap_or("com"),
        }
    }

    pub fn generate_audio(&self, text: &str) -> Result<Vec<u8>, String> {
        let len = text.len();
        if len > GOOGLE_TTS_MAX_CHARS {
            return Err(format!(
                "The text is too long. Max length is {}",
                GOOGLE_TTS_MAX_CHARS
            ));
        }
        let language = Languages::as_code(self.language.clone());
        let text = UrlTTS::fragmenter(text)?;
        let url = format!("https://translate.google.{}/translate_tts?ie=UTF-8&q={}&tl={}&total=1&idx=0&textlen={}&tl={}&client=tw-ob", self.tld, text.encoded, language, len, language);

        // From https://github.com/pndurette/gTTS/blob/15c891e336a947852296d7c1fb7d7ee485800c26/gtts/tts.py#L93
        // user_agent = "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/47.0.2526.106 Safari/537.36"

        let rep = minreq::get(url)
            .with_header("referer", "http://translate.google.com/")
            .with_header("user_agent", "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/47.0.2526.106 Safari/537.36")
            .send()
            .map_err(|e| format!("{}", e))?;

        let bytes = rep.as_bytes();

        Ok(bytes.to_vec())
    }
}
