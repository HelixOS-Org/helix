//! Internationalization and Localization for Helix UEFI Bootloader
//!
//! This module provides comprehensive localization support including
//! multiple languages, unicode handling, and date/time formatting.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Localization System                                 │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Language Support                              │   │
//! │  │  English │ French │ German │ Spanish │ Japanese │ Chinese      │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   String Resources                              │   │
//! │  │  Messages │ Errors │ Menu Items │ Labels                        │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Formatting                                    │   │
//! │  │  Numbers │ Dates │ Times │ Currency │ Units                     │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Unicode Support                               │   │
//! │  │  UTF-8 │ UTF-16 │ Normalization │ BiDi                          │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

// =============================================================================
// LANGUAGE CODES
// =============================================================================

/// ISO 639-1 Language Code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// English
    #[default]
    En,
    /// French
    Fr,
    /// German
    De,
    /// Spanish
    Es,
    /// Italian
    It,
    /// Portuguese
    Pt,
    /// Dutch
    Nl,
    /// Russian
    Ru,
    /// Japanese
    Ja,
    /// Chinese (Simplified)
    ZhCn,
    /// Chinese (Traditional)
    ZhTw,
    /// Korean
    Ko,
    /// Arabic
    Ar,
    /// Hebrew
    He,
    /// Polish
    Pl,
    /// Czech
    Cs,
    /// Hungarian
    Hu,
    /// Turkish
    Tr,
    /// Greek
    El,
    /// Thai
    Th,
    /// Vietnamese
    Vi,
    /// Indonesian
    Id,
    /// Hindi
    Hi,
    /// Swedish
    Sv,
    /// Norwegian
    No,
    /// Danish
    Da,
    /// Finnish
    Fi,
    /// Ukrainian
    Uk,
    /// Romanian
    Ro,
}

impl Language {
    /// Get ISO 639-1 code
    pub const fn code(&self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Fr => "fr",
            Language::De => "de",
            Language::Es => "es",
            Language::It => "it",
            Language::Pt => "pt",
            Language::Nl => "nl",
            Language::Ru => "ru",
            Language::Ja => "ja",
            Language::ZhCn => "zh-CN",
            Language::ZhTw => "zh-TW",
            Language::Ko => "ko",
            Language::Ar => "ar",
            Language::He => "he",
            Language::Pl => "pl",
            Language::Cs => "cs",
            Language::Hu => "hu",
            Language::Tr => "tr",
            Language::El => "el",
            Language::Th => "th",
            Language::Vi => "vi",
            Language::Id => "id",
            Language::Hi => "hi",
            Language::Sv => "sv",
            Language::No => "no",
            Language::Da => "da",
            Language::Fi => "fi",
            Language::Uk => "uk",
            Language::Ro => "ro",
        }
    }

    /// Get native language name
    pub const fn native_name(&self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Fr => "Français",
            Language::De => "Deutsch",
            Language::Es => "Español",
            Language::It => "Italiano",
            Language::Pt => "Português",
            Language::Nl => "Nederlands",
            Language::Ru => "Русский",
            Language::Ja => "日本語",
            Language::ZhCn => "简体中文",
            Language::ZhTw => "繁體中文",
            Language::Ko => "한국어",
            Language::Ar => "العربية",
            Language::He => "עברית",
            Language::Pl => "Polski",
            Language::Cs => "Čeština",
            Language::Hu => "Magyar",
            Language::Tr => "Türkçe",
            Language::El => "Ελληνικά",
            Language::Th => "ไทย",
            Language::Vi => "Tiếng Việt",
            Language::Id => "Bahasa Indonesia",
            Language::Hi => "हिन्दी",
            Language::Sv => "Svenska",
            Language::No => "Norsk",
            Language::Da => "Dansk",
            Language::Fi => "Suomi",
            Language::Uk => "Українська",
            Language::Ro => "Română",
        }
    }

    /// Get English name
    pub const fn english_name(&self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Fr => "French",
            Language::De => "German",
            Language::Es => "Spanish",
            Language::It => "Italian",
            Language::Pt => "Portuguese",
            Language::Nl => "Dutch",
            Language::Ru => "Russian",
            Language::Ja => "Japanese",
            Language::ZhCn => "Chinese (Simplified)",
            Language::ZhTw => "Chinese (Traditional)",
            Language::Ko => "Korean",
            Language::Ar => "Arabic",
            Language::He => "Hebrew",
            Language::Pl => "Polish",
            Language::Cs => "Czech",
            Language::Hu => "Hungarian",
            Language::Tr => "Turkish",
            Language::El => "Greek",
            Language::Th => "Thai",
            Language::Vi => "Vietnamese",
            Language::Id => "Indonesian",
            Language::Hi => "Hindi",
            Language::Sv => "Swedish",
            Language::No => "Norwegian",
            Language::Da => "Danish",
            Language::Fi => "Finnish",
            Language::Uk => "Ukrainian",
            Language::Ro => "Romanian",
        }
    }

    /// Check if RTL (right-to-left)
    pub const fn is_rtl(&self) -> bool {
        matches!(self, Self::Ar | Self::He)
    }

    /// Get script type
    pub const fn script(&self) -> Script {
        match self {
            Language::En
            | Language::Fr
            | Language::De
            | Language::Es
            | Language::It
            | Language::Pt
            | Language::Nl
            | Language::Pl
            | Language::Cs
            | Language::Hu
            | Language::Tr
            | Language::Vi
            | Language::Id
            | Language::Sv
            | Language::No
            | Language::Da
            | Language::Fi
            | Language::Ro => Script::Latin,
            Language::Ru | Language::Uk => Script::Cyrillic,
            Language::Ja => Script::Japanese,
            Language::ZhCn | Language::ZhTw => Script::Chinese,
            Language::Ko => Script::Korean,
            Language::Ar => Script::Arabic,
            Language::He => Script::Hebrew,
            Language::El => Script::Greek,
            Language::Th => Script::Thai,
            Language::Hi => Script::Devanagari,
        }
    }
}

/// Script type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    /// Latin alphabet
    Latin,
    /// Cyrillic alphabet
    Cyrillic,
    /// Greek alphabet
    Greek,
    /// Arabic script
    Arabic,
    /// Hebrew script
    Hebrew,
    /// Japanese (Hiragana, Katakana, Kanji)
    Japanese,
    /// Chinese characters
    Chinese,
    /// Korean Hangul
    Korean,
    /// Thai script
    Thai,
    /// Devanagari script
    Devanagari,
}

// =============================================================================
// COUNTRY CODES
// =============================================================================

/// ISO 3166-1 alpha-2 Country Code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    /// United States
    Us,
    /// United Kingdom
    Gb,
    /// Canada
    Ca,
    /// Australia
    Au,
    /// France
    Fr,
    /// Germany
    De,
    /// Spain
    Es,
    /// Italy
    It,
    /// Japan
    Jp,
    /// China
    Cn,
    /// Taiwan
    Tw,
    /// Korea (South)
    Kr,
    /// Russia
    Ru,
    /// Brazil
    Br,
    /// Mexico
    Mx,
    /// India
    In,
    /// Netherlands
    Nl,
    /// Belgium
    Be,
    /// Switzerland
    Ch,
    /// Austria
    At,
    /// Poland
    Pl,
    /// Sweden
    Se,
    /// Norway
    No,
    /// Denmark
    Dk,
    /// Finland
    Fi,
}

impl Country {
    /// Get ISO 3166-1 alpha-2 code
    pub const fn code(&self) -> &'static str {
        match self {
            Country::Us => "US",
            Country::Gb => "GB",
            Country::Ca => "CA",
            Country::Au => "AU",
            Country::Fr => "FR",
            Country::De => "DE",
            Country::Es => "ES",
            Country::It => "IT",
            Country::Jp => "JP",
            Country::Cn => "CN",
            Country::Tw => "TW",
            Country::Kr => "KR",
            Country::Ru => "RU",
            Country::Br => "BR",
            Country::Mx => "MX",
            Country::In => "IN",
            Country::Nl => "NL",
            Country::Be => "BE",
            Country::Ch => "CH",
            Country::At => "AT",
            Country::Pl => "PL",
            Country::Se => "SE",
            Country::No => "NO",
            Country::Dk => "DK",
            Country::Fi => "FI",
        }
    }
}

// =============================================================================
// LOCALE
// =============================================================================

/// Locale combining language and country
#[derive(Debug, Clone, Copy)]
pub struct Locale {
    /// Language
    pub language: Language,
    /// Country (optional)
    pub country: Option<Country>,
}

impl Locale {
    /// Create new locale
    pub const fn new(language: Language) -> Self {
        Self {
            language,
            country: None,
        }
    }

    /// Create with country
    pub const fn with_country(language: Language, country: Country) -> Self {
        Self {
            language,
            country: Some(country),
        }
    }

    /// US English
    pub const EN_US: Self = Self {
        language: Language::En,
        country: Some(Country::Us),
    };
    /// UK English
    pub const EN_GB: Self = Self {
        language: Language::En,
        country: Some(Country::Gb),
    };
    /// French (France)
    pub const FR_FR: Self = Self {
        language: Language::Fr,
        country: Some(Country::Fr),
    };
    /// German (Germany)
    pub const DE_DE: Self = Self {
        language: Language::De,
        country: Some(Country::De),
    };
    /// Spanish (Spain)
    pub const ES_ES: Self = Self {
        language: Language::Es,
        country: Some(Country::Es),
    };
    /// Japanese (Japan)
    pub const JA_JP: Self = Self {
        language: Language::Ja,
        country: Some(Country::Jp),
    };
    /// Chinese (China)
    pub const ZH_CN: Self = Self {
        language: Language::ZhCn,
        country: Some(Country::Cn),
    };
    /// Chinese (Taiwan)
    pub const ZH_TW: Self = Self {
        language: Language::ZhTw,
        country: Some(Country::Tw),
    };
}

impl Default for Locale {
    fn default() -> Self {
        Self::EN_US
    }
}

// =============================================================================
// STRING IDENTIFIERS
// =============================================================================

/// Boot message string IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootString {
    /// "Loading..."
    Loading,
    /// "Starting Helix OS..."
    StartingOs,
    /// "Press any key to continue..."
    PressAnyKey,
    /// "Boot menu"
    BootMenu,
    /// "Select boot device"
    SelectBootDevice,
    /// "Timeout: %d seconds"
    Timeout,
    /// "Default"
    Default,
    /// "Options"
    Options,
    /// "Exit"
    Exit,
    /// "Reboot"
    Reboot,
    /// "Shutdown"
    Shutdown,
    /// "Enter Setup"
    EnterSetup,
    /// "Boot from %s"
    BootFrom,
    /// "Loading kernel..."
    LoadingKernel,
    /// "Initializing memory..."
    InitMemory,
    /// "Starting services..."
    StartingServices,
    /// "Done"
    Done,
    /// "OK"
    Ok,
    /// "Cancel"
    Cancel,
    /// "Yes"
    Yes,
    /// "No"
    No,
    /// "Error"
    Error,
    /// "Warning"
    Warning,
    /// "Information"
    Information,
    /// "Secure Boot enabled"
    SecureBootEnabled,
    /// "Secure Boot disabled"
    SecureBootDisabled,
    /// "Verification failed"
    VerificationFailed,
    /// "Invalid signature"
    InvalidSignature,
    /// "File not found"
    FileNotFound,
    /// "Memory allocation failed"
    MemoryAllocationFailed,
    /// "Unknown error"
    UnknownError,
}

impl BootString {
    /// Get English translation
    pub const fn en(&self) -> &'static str {
        match self {
            BootString::Loading => "Loading...",
            BootString::StartingOs => "Starting Helix OS...",
            BootString::PressAnyKey => "Press any key to continue...",
            BootString::BootMenu => "Boot Menu",
            BootString::SelectBootDevice => "Select boot device",
            BootString::Timeout => "Timeout",
            BootString::Default => "Default",
            BootString::Options => "Options",
            BootString::Exit => "Exit",
            BootString::Reboot => "Reboot",
            BootString::Shutdown => "Shutdown",
            BootString::EnterSetup => "Enter Setup",
            BootString::BootFrom => "Boot from",
            BootString::LoadingKernel => "Loading kernel...",
            BootString::InitMemory => "Initializing memory...",
            BootString::StartingServices => "Starting services...",
            BootString::Done => "Done",
            BootString::Ok => "OK",
            BootString::Cancel => "Cancel",
            BootString::Yes => "Yes",
            BootString::No => "No",
            BootString::Error => "Error",
            BootString::Warning => "Warning",
            BootString::Information => "Information",
            BootString::SecureBootEnabled => "Secure Boot enabled",
            BootString::SecureBootDisabled => "Secure Boot disabled",
            BootString::VerificationFailed => "Verification failed",
            BootString::InvalidSignature => "Invalid signature",
            BootString::FileNotFound => "File not found",
            BootString::MemoryAllocationFailed => "Memory allocation failed",
            BootString::UnknownError => "Unknown error",
        }
    }

    /// Get French translation
    pub const fn fr(&self) -> &'static str {
        match self {
            BootString::Loading => "Chargement...",
            BootString::StartingOs => "Démarrage de Helix OS...",
            BootString::PressAnyKey => "Appuyez sur une touche pour continuer...",
            BootString::BootMenu => "Menu de démarrage",
            BootString::SelectBootDevice => "Sélectionner le périphérique de démarrage",
            BootString::Timeout => "Délai",
            BootString::Default => "Par défaut",
            BootString::Options => "Options",
            BootString::Exit => "Quitter",
            BootString::Reboot => "Redémarrer",
            BootString::Shutdown => "Arrêter",
            BootString::EnterSetup => "Entrer dans la configuration",
            BootString::BootFrom => "Démarrer depuis",
            BootString::LoadingKernel => "Chargement du noyau...",
            BootString::InitMemory => "Initialisation de la mémoire...",
            BootString::StartingServices => "Démarrage des services...",
            BootString::Done => "Terminé",
            BootString::Ok => "OK",
            BootString::Cancel => "Annuler",
            BootString::Yes => "Oui",
            BootString::No => "Non",
            BootString::Error => "Erreur",
            BootString::Warning => "Avertissement",
            BootString::Information => "Information",
            BootString::SecureBootEnabled => "Secure Boot activé",
            BootString::SecureBootDisabled => "Secure Boot désactivé",
            BootString::VerificationFailed => "Vérification échouée",
            BootString::InvalidSignature => "Signature invalide",
            BootString::FileNotFound => "Fichier non trouvé",
            BootString::MemoryAllocationFailed => "Allocation mémoire échouée",
            BootString::UnknownError => "Erreur inconnue",
        }
    }

    /// Get German translation
    pub const fn de(&self) -> &'static str {
        match self {
            BootString::Loading => "Wird geladen...",
            BootString::StartingOs => "Helix OS wird gestartet...",
            BootString::PressAnyKey => "Drücken Sie eine Taste zum Fortfahren...",
            BootString::BootMenu => "Startmenü",
            BootString::SelectBootDevice => "Startgerät auswählen",
            BootString::Timeout => "Zeitüberschreitung",
            BootString::Default => "Standard",
            BootString::Options => "Optionen",
            BootString::Exit => "Beenden",
            BootString::Reboot => "Neustart",
            BootString::Shutdown => "Herunterfahren",
            BootString::EnterSetup => "Setup aufrufen",
            BootString::BootFrom => "Starten von",
            BootString::LoadingKernel => "Kernel wird geladen...",
            BootString::InitMemory => "Speicher wird initialisiert...",
            BootString::StartingServices => "Dienste werden gestartet...",
            BootString::Done => "Fertig",
            BootString::Ok => "OK",
            BootString::Cancel => "Abbrechen",
            BootString::Yes => "Ja",
            BootString::No => "Nein",
            BootString::Error => "Fehler",
            BootString::Warning => "Warnung",
            BootString::Information => "Information",
            BootString::SecureBootEnabled => "Secure Boot aktiviert",
            BootString::SecureBootDisabled => "Secure Boot deaktiviert",
            BootString::VerificationFailed => "Verifizierung fehlgeschlagen",
            BootString::InvalidSignature => "Ungültige Signatur",
            BootString::FileNotFound => "Datei nicht gefunden",
            BootString::MemoryAllocationFailed => "Speicherzuweisung fehlgeschlagen",
            BootString::UnknownError => "Unbekannter Fehler",
        }
    }

    /// Get translation for language
    pub const fn get(&self, lang: Language) -> &'static str {
        match lang {
            Language::Fr => self.fr(),
            Language::De => self.de(),
            _ => self.en(), // Fallback to English
        }
    }
}

// =============================================================================
// NUMBER FORMATTING
// =============================================================================

/// Thousands separator style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThousandsSeparator {
    /// Comma (1,000,000)
    Comma,
    /// Period (1.000.000)
    Period,
    /// Space (1 000 000)
    Space,
    /// Apostrophe (1'000'000)
    Apostrophe,
    /// None (1000000)
    None,
}

/// Decimal separator style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalSeparator {
    /// Period (3.14)
    Period,
    /// Comma (3,14)
    Comma,
}

/// Number format configuration
#[derive(Debug, Clone, Copy)]
pub struct NumberFormat {
    /// Thousands separator
    pub thousands: ThousandsSeparator,
    /// Decimal separator
    pub decimal: DecimalSeparator,
    /// Minimum integer digits
    pub min_int_digits: u8,
    /// Minimum fraction digits
    pub min_frac_digits: u8,
    /// Maximum fraction digits
    pub max_frac_digits: u8,
}

impl NumberFormat {
    /// US English number format
    pub const EN_US: Self = Self {
        thousands: ThousandsSeparator::Comma,
        decimal: DecimalSeparator::Period,
        min_int_digits: 1,
        min_frac_digits: 0,
        max_frac_digits: 6,
    };

    /// French number format
    pub const FR_FR: Self = Self {
        thousands: ThousandsSeparator::Space,
        decimal: DecimalSeparator::Comma,
        min_int_digits: 1,
        min_frac_digits: 0,
        max_frac_digits: 6,
    };

    /// German number format
    pub const DE_DE: Self = Self {
        thousands: ThousandsSeparator::Period,
        decimal: DecimalSeparator::Comma,
        min_int_digits: 1,
        min_frac_digits: 0,
        max_frac_digits: 6,
    };

    /// Get format for locale
    pub const fn for_locale(locale: &Locale) -> Self {
        match locale.language {
            Language::Fr => Self::FR_FR,
            Language::De => Self::DE_DE,
            _ => Self::EN_US,
        }
    }
}

impl Default for NumberFormat {
    fn default() -> Self {
        Self::EN_US
    }
}

// =============================================================================
// DATE/TIME FORMATTING
// =============================================================================

/// Date format style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateFormat {
    /// MM/DD/YYYY (US)
    Mdy,
    /// DD/MM/YYYY (Europe)
    Dmy,
    /// YYYY-MM-DD (ISO 8601)
    #[default]
    Ymd,
    /// DD.MM.YYYY (German)
    DmyDot,
    /// YYYY/MM/DD (Japanese)
    YmdSlash,
}

impl DateFormat {
    /// Get format for locale
    pub const fn for_locale(locale: &Locale) -> Self {
        match locale.language {
            Language::Ja | Language::ZhCn | Language::ZhTw | Language::Ko => DateFormat::Ymd,
            Language::De | Language::Pl | Language::Cs | Language::Hu => DateFormat::DmyDot,
            Language::Fr | Language::Es | Language::It | Language::Pt | Language::Ru => {
                DateFormat::Dmy
            },
            _ => DateFormat::Mdy,
        }
    }
}

/// Time format style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeFormat {
    /// 12-hour with AM/PM
    Hour12,
    /// 24-hour
    #[default]
    Hour24,
}

impl TimeFormat {
    /// Get format for locale
    pub const fn for_locale(locale: &Locale) -> Self {
        match locale.language {
            Language::En => TimeFormat::Hour12,
            _ => TimeFormat::Hour24,
        }
    }
}

/// Day names
pub mod day_names {
    /// English day names
    pub const EN: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];

    /// English abbreviated day names
    pub const EN_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    /// French day names
    pub const FR: [&str; 7] = [
        "Dimanche", "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi",
    ];

    /// French abbreviated day names
    pub const FR_SHORT: [&str; 7] = ["Dim", "Lun", "Mar", "Mer", "Jeu", "Ven", "Sam"];

    /// German day names
    pub const DE: [&str; 7] = [
        "Sonntag",
        "Montag",
        "Dienstag",
        "Mittwoch",
        "Donnerstag",
        "Freitag",
        "Samstag",
    ];

    /// German abbreviated day names
    pub const DE_SHORT: [&str; 7] = ["So", "Mo", "Di", "Mi", "Do", "Fr", "Sa"];
}

/// Month names
pub mod month_names {
    /// English month names
    pub const EN: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];

    /// English abbreviated month names
    pub const EN_SHORT: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    /// French month names
    pub const FR: [&str; 12] = [
        "Janvier",
        "Février",
        "Mars",
        "Avril",
        "Mai",
        "Juin",
        "Juillet",
        "Août",
        "Septembre",
        "Octobre",
        "Novembre",
        "Décembre",
    ];

    /// French abbreviated month names
    pub const FR_SHORT: [&str; 12] = [
        "Jan", "Fév", "Mar", "Avr", "Mai", "Jun", "Jul", "Aoû", "Sep", "Oct", "Nov", "Déc",
    ];

    /// German month names
    pub const DE: [&str; 12] = [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ];

    /// German abbreviated month names
    pub const DE_SHORT: [&str; 12] = [
        "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
    ];
}

// =============================================================================
// SIZE UNITS
// =============================================================================

/// Binary size units (powers of 1024)
pub mod binary_units {
    /// Bytes
    pub const BYTE: &str = "B";
    /// Kibibytes
    pub const KIB: &str = "KiB";
    /// Mebibytes
    pub const MIB: &str = "MiB";
    /// Gibibytes
    pub const GIB: &str = "GiB";
    /// Tebibytes
    pub const TIB: &str = "TiB";
    /// Pebibytes
    pub const PIB: &str = "PiB";
}

/// SI size units (powers of 1000)
pub mod si_units {
    /// Bytes
    pub const BYTE: &str = "B";
    /// Kilobytes
    pub const KB: &str = "KB";
    /// Megabytes
    pub const MB: &str = "MB";
    /// Gigabytes
    pub const GB: &str = "GB";
    /// Terabytes
    pub const TB: &str = "TB";
    /// Petabytes
    pub const PB: &str = "PB";
}

/// Size formatting options
#[derive(Debug, Clone, Copy)]
pub struct SizeFormat {
    /// Use binary units (1024-based)
    pub binary: bool,
    /// Decimal places
    pub decimals: u8,
    /// Space between number and unit
    pub space: bool,
}

impl Default for SizeFormat {
    fn default() -> Self {
        Self {
            binary: true,
            decimals: 2,
            space: true,
        }
    }
}

// =============================================================================
// KEYBOARD LAYOUT
// =============================================================================

/// Keyboard layout identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardLayout {
    /// US QWERTY
    #[default]
    UsQwerty,
    /// UK QWERTY
    UkQwerty,
    /// German QWERTZ
    DeQwertz,
    /// French AZERTY
    FrAzerty,
    /// Spanish QWERTY
    EsQwerty,
    /// Italian QWERTY
    ItQwerty,
    /// Swiss German
    ChDe,
    /// Swiss French
    ChFr,
    /// Canadian French
    CaFr,
    /// Japanese
    Jp,
    /// Korean
    Kr,
    /// Russian
    Ru,
    /// Dvorak
    Dvorak,
    /// Colemak
    Colemak,
}

impl KeyboardLayout {
    /// Get default layout for locale
    pub const fn for_locale(locale: &Locale) -> Self {
        match (locale.language, locale.country) {
            (Language::En, Some(Country::Gb)) => Self::UkQwerty,
            (Language::De, Some(Country::Ch)) => Self::ChDe,
            (Language::De, _) => Self::DeQwertz,
            (Language::Fr, Some(Country::Ca)) => Self::CaFr,
            (Language::Fr, Some(Country::Ch)) => Self::ChFr,
            (Language::Fr, _) => Self::FrAzerty,
            (Language::Es, _) => Self::EsQwerty,
            (Language::It, _) => Self::ItQwerty,
            (Language::Ja, _) => Self::Jp,
            (Language::Ko, _) => Self::Kr,
            (Language::Ru, _) => Self::Ru,
            _ => Self::UsQwerty,
        }
    }
}

// =============================================================================
// UNICODE UTILITIES
// =============================================================================

/// Check if character is ASCII
pub const fn is_ascii(c: char) -> bool {
    (c as u32) < 128
}

/// Check if character is Latin Extended
pub const fn is_latin(c: char) -> bool {
    let cp = c as u32;
    cp < 0x0250 || (cp >= 0x1E00 && cp < 0x1F00)
}

/// Check if character is CJK
pub const fn is_cjk(c: char) -> bool {
    let cp = c as u32;
    (cp >= 0x4E00 && cp < 0x9FFF) ||  // CJK Unified
    (cp >= 0x3400 && cp < 0x4DBF) ||  // CJK Extension A
    (cp >= 0x3000 && cp < 0x303F) // CJK Symbols
}

/// Check if character is wide (takes 2 cells)
pub const fn is_wide(c: char) -> bool {
    let cp = c as u32;
    // CJK characters, full-width forms
    (cp >= 0x1100 && cp <= 0x115F) ||  // Hangul Jamo
    (cp >= 0x2E80 && cp <= 0x9FFF) ||  // CJK
    (cp >= 0xAC00 && cp <= 0xD7A3) ||  // Hangul Syllables
    (cp >= 0xF900 && cp <= 0xFAFF) ||  // CJK Compat
    (cp >= 0xFE10 && cp <= 0xFE1F) ||  // Vertical forms
    (cp >= 0xFF00 && cp <= 0xFF60) // Fullwidth forms
}

/// Character direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharDirection {
    /// Left-to-right
    LeftToRight,
    /// Right-to-left
    RightToLeft,
    /// Weak left-to-right
    WeakLeftToRight,
    /// Neutral
    Neutral,
}

/// Get character direction
pub const fn char_direction(c: char) -> CharDirection {
    let cp = c as u32;
    if (cp >= 0x0600 && cp <= 0x06FF) ||  // Arabic
       (cp >= 0x0590 && cp <= 0x05FF) ||  // Hebrew
       (cp >= 0xFB50 && cp <= 0xFDFF)
    // Arabic Presentation
    {
        CharDirection::RightToLeft
    } else if cp < 0x0080 && c.is_ascii_alphabetic() {
        CharDirection::LeftToRight
    } else if cp < 0x0080 && c.is_ascii_digit() {
        CharDirection::WeakLeftToRight
    } else {
        CharDirection::Neutral
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_language_codes() {
        assert_eq!(Language::En.code(), "en");
        assert_eq!(Language::Fr.code(), "fr");
        assert_eq!(Language::ZhCn.code(), "zh-CN");
    }

    #[test]
    fn test_rtl() {
        assert!(Language::Ar.is_rtl());
        assert!(Language::He.is_rtl());
        assert!(!Language::En.is_rtl());
    }

    #[test]
    fn test_boot_strings() {
        assert_eq!(BootString::Loading.en(), "Loading...");
        assert_eq!(BootString::Loading.fr(), "Chargement...");
        assert_eq!(BootString::Loading.de(), "Wird geladen...");
    }

    #[test]
    fn test_locale_default() {
        let locale = Locale::default();
        assert_eq!(locale.language, Language::En);
    }

    #[test]
    fn test_unicode() {
        assert!(is_ascii('A'));
        assert!(!is_ascii('é'));
        assert!(is_cjk('中'));
        assert!(is_wide('中'));
        assert!(!is_wide('A'));
    }

    #[test]
    fn test_char_direction() {
        assert_eq!(char_direction('A'), CharDirection::LeftToRight);
        assert_eq!(char_direction(' '), CharDirection::Neutral);
    }
}
