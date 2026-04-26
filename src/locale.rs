use crate::types::Units;

pub fn detect() -> (Option<Units>, Option<&'static str>) {
    let Some(bcp47) = sys_locale::get_locale() else {
        return (None, None);
    };
    let units = units_for_locale(&bcp47);
    let lang = lang_for_locale(&bcp47);
    (Some(units), Some(lang))
}

pub fn units_for_locale(bcp47: &str) -> Units {
    if bcp47.starts_with("en-US")
        || bcp47.starts_with("en-LR")
        || bcp47.starts_with("my-")
        || bcp47 == "en_US"
        || bcp47.starts_with("en_US.")
    {
        Units::Imperial
    } else if bcp47.starts_with("en-GB") || bcp47 == "en_GB" || bcp47.starts_with("en_GB.") {
        Units::British
    } else {
        Units::Metric
    }
}

pub fn lang_for_locale(bcp47: &str) -> &'static str {
    let parts: Vec<&str> = bcp47
        .split(|c| c == '-' || c == '_' || c == '.')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.first().is_some_and(|p| p.eq_ignore_ascii_case("pt")) {
        // pt-BR vs pt-PT (and other regions): only Brazil uses the pt-br UI + pt_BR chrono;
        // Portugal, Africa, etc. use pt-pt + pt_PT. Undocumented `pt` alone defaults to pt-br.
        return match parts.get(1) {
            None => "pt-br",
            Some(r) if r.eq_ignore_ascii_case("br") => "pt-br",
            _ => "pt-pt",
        };
    }

    if parts.first().is_some_and(|p| p.eq_ignore_ascii_case("es")) {
        // Spain (region ES) vs Latin America / `es-419` / undetermined `es` + chrono `es_MX` for 419.
        return match parts.get(1) {
            None => "es-419",
            Some(r) if r.eq_ignore_ascii_case("es") => "es-es", // ISO 3166-1 alpha-2 ES (Spain)
            Some(r) if *r == "419" => "es-419",
            _ => "es-419",
        };
    }

    if parts.first().is_some_and(|p| p.eq_ignore_ascii_case("fr")) {
        // France/Europe (fr-fr) vs Canada (fr-ca) + chrono.
        return match parts.get(1) {
            Some(r) if r.eq_ignore_ascii_case("ca") => "fr-ca",
            _ => "fr-fr",
        };
    }

    let prefix = parts
        .first()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    match prefix.as_str() {
        "ca" => "ca",
        "cs" => "cs",
        "da" => "da",
        "de" => "de",
        "el" => "el",
        "fi" => "fi",
        "hr" => "hr",
        "hu" => "hu",
        "it" => "it",
        "nb" => "nb",
        "nl" => "nl",
        "pl" => "pl",
        "ro" => "ro",
        "ru" => "ru",
        "sk" => "sk",
        "sv" => "sv",
        "tr" => "tr",
        "uk" => "uk",
        _ => "en",
    }
}

pub fn chrono_locale(lang: &str) -> chrono::Locale {
    match lang {
        "ca" => chrono::Locale::ca_ES,
        "cs" => chrono::Locale::cs_CZ,
        "da" => chrono::Locale::da_DK,
        "de" => chrono::Locale::de_DE,
        "el" => chrono::Locale::el_GR,
        "es-es" => chrono::Locale::es_ES,
        "es-419" => chrono::Locale::es_MX,
        "fi" => chrono::Locale::fi_FI,
        "fr-fr" => chrono::Locale::fr_FR,
        "fr-ca" => chrono::Locale::fr_CA,
        "hr" => chrono::Locale::hr_HR,
        "hu" => chrono::Locale::hu_HU,
        "it" => chrono::Locale::it_IT,
        "nb" => chrono::Locale::nb_NO,
        "nl" => chrono::Locale::nl_NL,
        "pl" => chrono::Locale::pl_PL,
        "pt-br" => chrono::Locale::pt_BR,
        "pt-pt" => chrono::Locale::pt_PT,
        "ro" => chrono::Locale::ro_RO,
        "ru" => chrono::Locale::ru_RU,
        "sk" => chrono::Locale::sk_SK,
        "sv" => chrono::Locale::sv_SE,
        "tr" => chrono::Locale::tr_TR,
        "uk" => chrono::Locale::uk_UA,
        _ => chrono::Locale::en_US,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_us_locale_is_imperial() {
        assert_eq!(units_for_locale("en-US"), Units::Imperial);
        assert_eq!(units_for_locale("en_US.UTF-8"), Units::Imperial);
    }

    #[test]
    fn units_lr_locale_is_imperial() {
        assert_eq!(units_for_locale("en-LR"), Units::Imperial);
    }

    #[test]
    fn units_myanmar_is_imperial() {
        assert_eq!(units_for_locale("my-MM"), Units::Imperial);
    }

    #[test]
    fn units_gb_locale_is_british() {
        assert_eq!(units_for_locale("en-GB"), Units::British);
        assert_eq!(units_for_locale("en_GB.UTF-8"), Units::British);
    }

    #[test]
    fn units_metric_fallback() {
        assert_eq!(units_for_locale("pl-PL"), Units::Metric);
        assert_eq!(units_for_locale("de-DE"), Units::Metric);
        assert_eq!(units_for_locale("fr-FR"), Units::Metric);
        assert_eq!(units_for_locale("ja-JP"), Units::Metric);
    }

    #[test]
    fn lang_polish() {
        assert_eq!(lang_for_locale("pl-PL"), "pl");
        assert_eq!(lang_for_locale("pl_PL.UTF-8"), "pl");
    }

    #[test]
    fn lang_german() {
        assert_eq!(lang_for_locale("de-DE"), "de");
    }

    #[test]
    fn lang_spanish_regions() {
        assert_eq!(lang_for_locale("es-ES"), "es-es");
        assert_eq!(lang_for_locale("es_MX.UTF-8"), "es-419");
        assert_eq!(lang_for_locale("es-419"), "es-419");
        assert_eq!(lang_for_locale("es"), "es-419");
    }

    #[test]
    fn lang_russian() {
        assert_eq!(lang_for_locale("ru-RU"), "ru");
    }

    #[test]
    fn lang_ukrainian() {
        assert_eq!(lang_for_locale("uk-UA"), "uk");
    }

    #[test]
    fn lang_french_regions() {
        assert_eq!(lang_for_locale("fr-FR"), "fr-fr");
        assert_eq!(lang_for_locale("fr_FR.UTF-8"), "fr-fr");
        assert_eq!(lang_for_locale("fr-CA"), "fr-ca");
        assert_eq!(lang_for_locale("fr-BE"), "fr-fr");
    }

    #[test]
    fn lang_norwegian_bokmal() {
        assert_eq!(lang_for_locale("nb-NO"), "nb");
    }

    #[test]
    fn lang_fallback_to_english() {
        assert_eq!(lang_for_locale("ja-JP"), "en");
        assert_eq!(lang_for_locale("en-US"), "en");
        assert_eq!(lang_for_locale("en-GB"), "en");
    }

    #[test]
    fn chrono_locale_mapping() {
        assert_eq!(chrono_locale("pl"), chrono::Locale::pl_PL);
        assert_eq!(chrono_locale("de"), chrono::Locale::de_DE);
        assert_eq!(chrono_locale("es-es"), chrono::Locale::es_ES);
        assert_eq!(chrono_locale("es-419"), chrono::Locale::es_MX);
        assert_eq!(chrono_locale("ru"), chrono::Locale::ru_RU);
        assert_eq!(chrono_locale("uk"), chrono::Locale::uk_UA);
        assert_eq!(chrono_locale("fr-fr"), chrono::Locale::fr_FR);
        assert_eq!(chrono_locale("fr-ca"), chrono::Locale::fr_CA);
        assert_eq!(chrono_locale("pt-br"), chrono::Locale::pt_BR);
        assert_eq!(chrono_locale("pt-pt"), chrono::Locale::pt_PT);
        assert_eq!(chrono_locale("nb"), chrono::Locale::nb_NO);
        assert_eq!(chrono_locale("en"), chrono::Locale::en_US);
    }

    #[test]
    fn lang_portuguese_regions() {
        assert_eq!(lang_for_locale("pt-BR"), "pt-br");
        assert_eq!(lang_for_locale("pt_BR.UTF-8"), "pt-br");
        assert_eq!(lang_for_locale("pt-PT"), "pt-pt");
        assert_eq!(lang_for_locale("pt-PT@euro"), "pt-pt");
        assert_eq!(lang_for_locale("pt"), "pt-br");
        assert_eq!(lang_for_locale("pt-AO"), "pt-pt");
    }
}
