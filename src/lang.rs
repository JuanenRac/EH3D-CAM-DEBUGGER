//! The words of the window in the eight languages of the repository (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal).

/// The languages of the window, in the order the lists of words below follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    De,
    Fr,
    It,
    Ja,
    Zh,
    Pt,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The two letters of the language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Pt => "pt",
        }
    }

    /// The name of the language in the language itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Pt => "Português",
        }
    }

    /// The language of a code such as `es`, `es_ES.UTF-8` or `pt-PT`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let two: String = code
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == two)
    }

    /// The language of the person from the variables of the system; English when it is none of the eight.
    pub fn from_env() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if !v.is_empty() {
                    return Lang::from_code(&v).unwrap_or(Lang::En);
                }
            }
        }
        Lang::En
    }
}

/// Puts the values in the places `{0}`, `{1}`... of a text.
pub fn fill(template: &str, values: &[&str]) -> String {
    let mut out = template.to_string();
    for (k, v) in values.iter().enumerate() {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Every word and sentence of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    Language,
    Fit,
    Cancel,
    Up,
    AllFiles,
    EmptyFolder,
    ZoomHint,
    NoFile,
    Ready,
    Failed,
    BadValue,
    Stale,
    CursorPlain,
    OpenProgram,
    Analyse,
    TitleOpen,
    SectionFile,
    SectionMachine,
    SectionLimits,
    SectionChecks,
    SectionFindings,
    SectionStep,
    SectionSteps,
    Accel,
    Rapid,
    FeedDefault,
    Dialect,
    DialectGrbl,
    DialectMarlin,
    DialectPlanetCnc,
    Laser,
    Strict,
    TableWidth,
    TableDepth,
    Surface,
    MinZ,
    MaxFeed,
    Ignore,
    Clamps,
    First,
    Previous,
    Next,
    Last,
    Play,
    Pause,
    LegendCut,
    LegendRapid,
    LegendLater,
    LegendAlert,
    LegendWarning,
    LegendClamp,
    LegendTable,
    DropHint,
    NoMoves,
    NoFindings,
    Overview,
    TimeLine,
    WaitsLine,
    FindingLine,
    StepOf,
    StepLine,
    StepFrom,
    StepTo,
    StepKindRapid,
    StepKindCut,
    StepFeedOn,
    StepFeedOff,
    ProgramOpened,
    Analysed,
    NeedProgram,
    Truncated,
}

impl Msg {
    pub const ALL: [Msg; 71] = [
        Msg::Language,
        Msg::Fit,
        Msg::Cancel,
        Msg::Up,
        Msg::AllFiles,
        Msg::EmptyFolder,
        Msg::ZoomHint,
        Msg::NoFile,
        Msg::Ready,
        Msg::Failed,
        Msg::BadValue,
        Msg::Stale,
        Msg::CursorPlain,
        Msg::OpenProgram,
        Msg::Analyse,
        Msg::TitleOpen,
        Msg::SectionFile,
        Msg::SectionMachine,
        Msg::SectionLimits,
        Msg::SectionChecks,
        Msg::SectionFindings,
        Msg::SectionStep,
        Msg::SectionSteps,
        Msg::Accel,
        Msg::Rapid,
        Msg::FeedDefault,
        Msg::Dialect,
        Msg::DialectGrbl,
        Msg::DialectMarlin,
        Msg::DialectPlanetCnc,
        Msg::Laser,
        Msg::Strict,
        Msg::TableWidth,
        Msg::TableDepth,
        Msg::Surface,
        Msg::MinZ,
        Msg::MaxFeed,
        Msg::Ignore,
        Msg::Clamps,
        Msg::First,
        Msg::Previous,
        Msg::Next,
        Msg::Last,
        Msg::Play,
        Msg::Pause,
        Msg::LegendCut,
        Msg::LegendRapid,
        Msg::LegendLater,
        Msg::LegendAlert,
        Msg::LegendWarning,
        Msg::LegendClamp,
        Msg::LegendTable,
        Msg::DropHint,
        Msg::NoMoves,
        Msg::NoFindings,
        Msg::Overview,
        Msg::TimeLine,
        Msg::WaitsLine,
        Msg::FindingLine,
        Msg::StepOf,
        Msg::StepLine,
        Msg::StepFrom,
        Msg::StepTo,
        Msg::StepKindRapid,
        Msg::StepKindCut,
        Msg::StepFeedOn,
        Msg::StepFeedOff,
        Msg::ProgramOpened,
        Msg::Analysed,
        Msg::NeedProgram,
        Msg::Truncated,
    ];

    /// The words in the order English, Spanish, German, French, Italian, Japanese, Chinese, Portuguese.
    fn words(self) -> [&'static str; 8] {
        match self {
            Msg::Language => [
                "Language", "Idioma", "Sprache", "Langue", "Lingua", "言語", "语言", "Idioma",
            ],
            Msg::Fit => [
                "Fit to window",
                "Ajustar a la ventana",
                "An Fenster anpassen",
                "Ajuster à la fenêtre",
                "Adatta alla finestra",
                "ウィンドウに合わせる",
                "适应窗口",
                "Ajustar à janela",
            ],
            Msg::Cancel => [
                "Cancel",
                "Cancelar",
                "Abbrechen",
                "Annuler",
                "Annulla",
                "キャンセル",
                "取消",
                "Cancelar",
            ],
            Msg::Up => [
                "Up",
                "Subir",
                "Hoch",
                "Dossier parent",
                "Su",
                "上へ",
                "上一级",
                "Subir",
            ],
            Msg::AllFiles => [
                "All files",
                "Todos los ficheros",
                "Alle Dateien",
                "Tous les fichiers",
                "Tutti i file",
                "すべてのファイル",
                "所有文件",
                "Todos os ficheiros",
            ],
            Msg::EmptyFolder => [
                "No files of this kind here",
                "No hay ficheros de este tipo aquí",
                "Hier gibt es keine Dateien dieser Art",
                "Aucun fichier de ce type ici",
                "Nessun file di questo tipo qui",
                "この種類のファイルはありません",
                "此处没有这类文件",
                "Não há ficheiros deste tipo aqui",
            ],
            Msg::ZoomHint => [
                "Wheel: zoom - drag: move - double click: fit",
                "Rueda: zoom - arrastrar: mover - doble clic: ajustar",
                "Rad: Zoom - Ziehen: verschieben - Doppelklick: anpassen",
                "Molette : zoom - glisser : déplacer - double clic : ajuster",
                "Rotella: zoom - trascina: sposta - doppio clic: adatta",
                "ホイール: ズーム - ドラッグ: 移動 - ダブルクリック: 全体表示",
                "滚轮：缩放 - 拖动：平移 - 双击：适应窗口",
                "Roda: zoom - arrastar: mover - duplo clique: ajustar",
            ],
            Msg::NoFile => [
                "(no file)",
                "(sin fichero)",
                "(keine Datei)",
                "(aucun fichier)",
                "(nessun file)",
                "（ファイルなし）",
                "（无文件）",
                "(sem ficheiro)",
            ],
            Msg::Ready => [
                "Ready",
                "Listo",
                "Bereit",
                "Prêt",
                "Pronto",
                "準備完了",
                "就绪",
                "Pronto",
            ],
            Msg::Failed => ["{0}", "{0}", "{0}", "{0}", "{0}", "{0}", "{0}", "{0}"],
            Msg::BadValue => [
                "{0}: this value is not valid",
                "{0}: este valor no es válido",
                "{0}: Dieser Wert ist ungültig",
                "{0} : cette valeur n'est pas valide",
                "{0}: questo valore non è valido",
                "{0}: この値は無効です",
                "{0}：该值无效",
                "{0}: este valor não é válido",
            ],
            Msg::Stale => [
                "The settings changed: compute again",
                "Los ajustes cambiaron: vuelve a calcular",
                "Die Einstellungen haben sich geändert: neu berechnen",
                "Les réglages ont changé : recalculez",
                "Le impostazioni sono cambiate: calcola di nuovo",
                "設定が変わりました。もう一度計算してください",
                "设置已更改：请重新计算",
                "As definições mudaram: calcule outra vez",
            ],
            Msg::CursorPlain => [
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
            ],
            Msg::OpenProgram => [
                "Open G-code...",
                "Abrir G-code...",
                "G-Code öffnen...",
                "Ouvrir un G-code...",
                "Apri G-code...",
                "G コードを開く...",
                "打开 G 代码...",
                "Abrir G-code...",
            ],
            Msg::Analyse => [
                "Analyse",
                "Analizar",
                "Analysieren",
                "Analyser",
                "Analizza",
                "解析",
                "分析",
                "Analisar",
            ],
            Msg::TitleOpen => [
                "Open a G-code program",
                "Abrir un programa G-code",
                "G-Code-Programm öffnen",
                "Ouvrir un programme G-code",
                "Apri un programma G-code",
                "G コード プログラムを開く",
                "打开 G 代码程序",
                "Abrir um programa G-code",
            ],
            Msg::SectionFile => [
                "Program",
                "Programa",
                "Programm",
                "Programme",
                "Programma",
                "プログラム",
                "程序",
                "Programa",
            ],
            Msg::SectionMachine => [
                "Machine", "Máquina", "Maschine", "Machine", "Macchina", "機械", "机器", "Máquina",
            ],
            Msg::SectionLimits => [
                "Limits and clamps",
                "Límites y sujeciones",
                "Grenzen und Spannmittel",
                "Limites et brides",
                "Limiti e staffe",
                "範囲とクランプ",
                "范围与夹具",
                "Limites e fixações",
            ],
            Msg::SectionChecks => [
                "Checks",
                "Comprobaciones",
                "Prüfungen",
                "Vérifications",
                "Controlli",
                "検査",
                "检查",
                "Verificações",
            ],
            Msg::SectionFindings => [
                "Findings",
                "Hallazgos",
                "Befunde",
                "Constats",
                "Rilievi",
                "検出結果",
                "发现",
                "Achados",
            ],
            Msg::SectionStep => [
                "The step",
                "El paso",
                "Der Schritt",
                "L'étape",
                "Il passo",
                "ステップ",
                "步骤",
                "O passo",
            ],
            Msg::SectionSteps => [
                "Moves",
                "Movimientos",
                "Bewegungen",
                "Mouvements",
                "Movimenti",
                "移動",
                "移动",
                "Movimentos",
            ],
            Msg::Accel => [
                "Acceleration (mm/s²)",
                "Aceleración (mm/s²)",
                "Beschleunigung (mm/s²)",
                "Accélération (mm/s²)",
                "Accelerazione (mm/s²)",
                "加速度 (mm/s²)",
                "加速度 (mm/s²)",
                "Aceleração (mm/s²)",
            ],
            Msg::Rapid => [
                "Rapid speed (mm/min)",
                "Velocidad rápida (mm/min)",
                "Eilgang (mm/min)",
                "Vitesse rapide (mm/min)",
                "Velocità rapida (mm/min)",
                "早送り速度 (mm/min)",
                "快速移动速度 (mm/min)",
                "Velocidade rápida (mm/min)",
            ],
            Msg::FeedDefault => [
                "Feed when a cut has none (mm/min)",
                "Avance si un corte no tiene (mm/min)",
                "Vorschub, wenn ein Schnitt keinen hat (mm/min)",
                "Avance si une coupe n'en a pas (mm/min)",
                "Avanzamento se un taglio non ne ha (mm/min)",
                "切削に送りがないときの送り速度 (mm/min)",
                "切削没有进给时使用的速度 (mm/min)",
                "Avanço se um corte não tiver (mm/min)",
            ],
            Msg::Dialect => [
                "Dialect of the G-code",
                "Dialecto del G-code",
                "G-Code-Dialekt",
                "Dialecte du G-code",
                "Dialetto del G-code",
                "G コードの方言",
                "G 代码方言",
                "Dialeto do G-code",
            ],
            Msg::DialectGrbl => [
                "GRBL", "GRBL", "GRBL", "GRBL", "GRBL", "GRBL", "GRBL", "GRBL",
            ],
            Msg::DialectMarlin => [
                "Marlin", "Marlin", "Marlin", "Marlin", "Marlin", "Marlin", "Marlin", "Marlin",
            ],
            Msg::DialectPlanetCnc => [
                "PlanetCNC",
                "PlanetCNC",
                "PlanetCNC",
                "PlanetCNC",
                "PlanetCNC",
                "PlanetCNC",
                "PlanetCNC",
                "PlanetCNC",
            ],
            Msg::Laser => [
                "Laser mode (M3 or M4 with S0 is not burning)",
                "Modo láser (M3 o M4 con S0 no quema)",
                "Lasermodus (M3 oder M4 mit S0 brennt nicht)",
                "Mode laser (M3 ou M4 avec S0 ne brûle pas)",
                "Modalità laser (M3 o M4 con S0 non brucia)",
                "レーザー モード（S0 の M3 または M4 は照射しない）",
                "激光模式（M3 或 M4 且 S0 时不烧蚀）",
                "Modo laser (M3 ou M4 com S0 não queima)",
            ],
            Msg::Strict => [
                "Strict (every warning is an alert)",
                "Estricto (toda advertencia es una alerta)",
                "Streng (jede Warnung ist ein Alarm)",
                "Strict (tout avertissement est une alerte)",
                "Rigoroso (ogni avviso è un allarme)",
                "厳格（すべての警告をアラートにする）",
                "严格（所有警告都视为警报）",
                "Estrito (todo aviso é um alerta)",
            ],
            Msg::TableWidth => [
                "Table width X (mm, empty: not checked)",
                "Ancho de la mesa X (mm, vacío: no se comprueba)",
                "Tischbreite X (mm, leer: nicht geprüft)",
                "Largeur de la table X (mm, vide : non vérifiée)",
                "Larghezza del piano X (mm, vuoto: non controllata)",
                "テーブル幅 X (mm、空欄：検査しない)",
                "工作台宽度 X (mm，留空则不检查)",
                "Largura da mesa X (mm, vazio: não se verifica)",
            ],
            Msg::TableDepth => [
                "Table depth Y (mm, empty: not checked)",
                "Fondo de la mesa Y (mm, vacío: no se comprueba)",
                "Tischtiefe Y (mm, leer: nicht geprüft)",
                "Profondeur de la table Y (mm, vide : non vérifiée)",
                "Profondità del piano Y (mm, vuoto: non controllata)",
                "テーブル奥行 Y (mm、空欄：検査しない)",
                "工作台深度 Y (mm，留空则不检查)",
                "Profundidade da mesa Y (mm, vazio: não se verifica)",
            ],
            Msg::Surface => [
                "Top of the material, Z (mm)",
                "Cara superior del material, Z (mm)",
                "Oberseite des Materials, Z (mm)",
                "Dessus de la matière, Z (mm)",
                "Faccia superiore del materiale, Z (mm)",
                "材料の上面 Z (mm)",
                "材料上表面 Z (mm)",
                "Face superior do material, Z (mm)",
            ],
            Msg::MinZ => [
                "Lowest Z of the machine (mm, empty: not checked)",
                "Z más baja de la máquina (mm, vacío: no se comprueba)",
                "Tiefstes Z der Maschine (mm, leer: nicht geprüft)",
                "Z le plus bas de la machine (mm, vide : non vérifié)",
                "Z più bassa della macchina (mm, vuoto: non controllata)",
                "機械の最低 Z (mm、空欄：検査しない)",
                "机器最低 Z (mm，留空则不检查)",
                "Z mais baixo da máquina (mm, vazio: não se verifica)",
            ],
            Msg::MaxFeed => [
                "Highest cutting feed (mm/min, empty: not checked)",
                "Avance de corte máximo (mm/min, vacío: no se comprueba)",
                "Höchster Schnittvorschub (mm/min, leer: nicht geprüft)",
                "Avance de coupe maximal (mm/min, vide : non vérifié)",
                "Avanzamento di taglio massimo (mm/min, vuoto: non controllato)",
                "最大の切削送り (mm/min、空欄：検査しない)",
                "最大切削进给 (mm/min，留空则不检查)",
                "Avanço de corte máximo (mm/min, vazio: não se verifica)",
            ],
            Msg::Ignore => [
                "Notices of the reader to leave out (codes separated by commas)",
                "Avisos del lector que se omiten (códigos separados por comas)",
                "Hinweise des Lesers, die ausgelassen werden (Codes durch Kommas getrennt)",
                "Avis du lecteur à omettre (codes séparés par des virgules)",
                "Avvisi del lettore da omettere (codici separati da virgole)",
                "省略するリーダーの通知（コードをカンマ区切り）",
                "要忽略的读取器提示（代码以逗号分隔）",
                "Avisos do leitor a omitir (códigos separados por vírgulas)",
            ],
            Msg::Clamps => [
                "Clamps, one on each line: X0,Y0,X1,Y1,HEIGHT",
                "Sujeciones, una por línea: X0,Y0,X1,Y1,ALTURA",
                "Spannmittel, eines pro Zeile: X0,Y0,X1,Y1,HÖHE",
                "Brides, une par ligne : X0,Y0,X1,Y1,HAUTEUR",
                "Staffe, una per riga: X0,Y0,X1,Y1,ALTEZZA",
                "クランプ（1 行に 1 つ）：X0,Y0,X1,Y1,高さ",
                "夹具，每行一个：X0,Y0,X1,Y1,高度",
                "Fixações, uma por linha: X0,Y0,X1,Y1,ALTURA",
            ],
            Msg::First => [
                "First",
                "Primero",
                "Erster",
                "Premier",
                "Primo",
                "最初",
                "第一步",
                "Primeiro",
            ],
            Msg::Previous => [
                "Previous",
                "Anterior",
                "Zurück",
                "Précédent",
                "Precedente",
                "前へ",
                "上一步",
                "Anterior",
            ],
            Msg::Next => [
                "Next",
                "Siguiente",
                "Weiter",
                "Suivant",
                "Successivo",
                "次へ",
                "下一步",
                "Seguinte",
            ],
            Msg::Last => [
                "Last",
                "Último",
                "Letzter",
                "Dernier",
                "Ultimo",
                "最後",
                "最后一步",
                "Último",
            ],
            Msg::Play => [
                "Play",
                "Reproducir",
                "Abspielen",
                "Lire",
                "Riproduci",
                "再生",
                "播放",
                "Reproduzir",
            ],
            Msg::Pause => [
                "Pause",
                "Pausa",
                "Pause",
                "Pause",
                "Pausa",
                "一時停止",
                "暂停",
                "Pausa",
            ],
            Msg::LegendCut => [
                "Cutting move",
                "Movimiento de corte",
                "Schnittbewegung",
                "Mouvement de coupe",
                "Movimento di taglio",
                "切削移動",
                "切削移动",
                "Movimento de corte",
            ],
            Msg::LegendRapid => [
                "Rapid move",
                "Movimiento rápido",
                "Eilgangbewegung",
                "Mouvement rapide",
                "Movimento rapido",
                "早送り移動",
                "快速移动",
                "Movimento rápido",
            ],
            Msg::LegendLater => [
                "Not reached yet",
                "Todavía no alcanzado",
                "Noch nicht erreicht",
                "Pas encore atteint",
                "Non ancora raggiunto",
                "未到達",
                "尚未到达",
                "Ainda não alcançado",
            ],
            Msg::LegendAlert => [
                "Alert",
                "Alerta",
                "Alarm",
                "Alerte",
                "Allarme",
                "アラート",
                "警报",
                "Alerta",
            ],
            Msg::LegendWarning => [
                "Warning",
                "Advertencia",
                "Warnung",
                "Avertissement",
                "Avviso",
                "警告",
                "警告",
                "Aviso",
            ],
            Msg::LegendClamp => [
                "Clamp",
                "Sujeción",
                "Spannmittel",
                "Bride",
                "Staffa",
                "クランプ",
                "夹具",
                "Fixação",
            ],
            Msg::LegendTable => [
                "Edge of the table",
                "Borde de la mesa",
                "Tischrand",
                "Bord de la table",
                "Bordo del piano",
                "テーブルの縁",
                "工作台边缘",
                "Limite da mesa",
            ],
            Msg::DropHint => [
                "Open or drop a G-code program here",
                "Abre o suelta aquí un programa G-code",
                "Öffnen oder legen Sie hier ein G-Code-Programm ab",
                "Ouvrez ou déposez ici un programme G-code",
                "Apri o trascina qui un programma G-code",
                "G コード プログラムをここに開く、またはドロップしてください",
                "在此打开或拖入 G 代码程序",
                "Abra ou largue aqui um programa G-code",
            ],
            Msg::NoMoves => [
                "The program has no moves to show",
                "El programa no tiene movimientos que mostrar",
                "Das Programm hat keine Bewegungen zum Anzeigen",
                "Le programme n'a aucun mouvement à montrer",
                "Il programma non ha movimenti da mostrare",
                "表示できる移動がありません",
                "程序没有可显示的移动",
                "O programa não tem movimentos para mostrar",
            ],
            Msg::NoFindings => [
                "No findings",
                "Sin hallazgos",
                "Keine Befunde",
                "Aucun constat",
                "Nessun rilievo",
                "検出結果なし",
                "没有发现",
                "Sem achados",
            ],
            Msg::Overview => [
                "{0} moves: {1} mm of cut, {2} mm of rapid",
                "{0} movimientos: {1} mm de corte, {2} mm de rápido",
                "{0} Bewegungen: {1} mm Schnitt, {2} mm Eilgang",
                "{0} mouvements : {1} mm de coupe, {2} mm de rapide",
                "{0} movimenti: {1} mm di taglio, {2} mm di rapido",
                "{0} 件の移動：切削 {1} mm、早送り {2} mm",
                "{0} 次移动：切削 {1} mm，快速 {2} mm",
                "{0} movimentos: {1} mm de corte, {2} mm de rápido",
            ],
            Msg::TimeLine => [
                "Time (upper bound): {0} (cutting {1}, rapid {2})",
                "Tiempo (cota superior): {0} (corte {1}, rápido {2})",
                "Zeit (Obergrenze): {0} (Schnitt {1}, Eilgang {2})",
                "Temps (borne supérieure) : {0} (coupe {1}, rapide {2})",
                "Tempo (limite superiore): {0} (taglio {1}, rapido {2})",
                "時間（上限）：{0}（切削 {1}、早送り {2}）",
                "时间（上限）：{0}（切削 {1}，快速 {2}）",
                "Tempo (limite superior): {0} (corte {1}, rápido {2})",
            ],
            Msg::WaitsLine => [
                "Waits (G4): {0} (with the waits: {1})",
                "Esperas (G4): {0} (con las esperas: {1})",
                "Wartezeiten (G4): {0} (mit den Wartezeiten: {1})",
                "Attentes (G4) : {0} (avec les attentes : {1})",
                "Attese (G4): {0} (con le attese: {1})",
                "待機（G4）：{0}（待機を含めると {1}）",
                "等待（G4）：{0}（含等待：{1}）",
                "Esperas (G4): {0} (com as esperas: {1})",
            ],
            Msg::FindingLine => [
                "{0} line {1}: {2}",
                "{0} línea {1}: {2}",
                "{0} Zeile {1}: {2}",
                "{0} ligne {1} : {2}",
                "{0} riga {1}: {2}",
                "{0} {1} 行目：{2}",
                "{0} 第 {1} 行：{2}",
                "{0} linha {1}: {2}",
            ],
            Msg::StepOf => [
                "Step {0} of {1}",
                "Paso {0} de {1}",
                "Schritt {0} von {1}",
                "Étape {0} sur {1}",
                "Passo {0} di {1}",
                "ステップ {0} / {1}",
                "第 {0} 步，共 {1} 步",
                "Passo {0} de {1}",
            ],
            Msg::StepLine => [
                "Line {0}: {1}",
                "Línea {0}: {1}",
                "Zeile {0}: {1}",
                "Ligne {0} : {1}",
                "Riga {0}: {1}",
                "{0} 行目：{1}",
                "第 {0} 行：{1}",
                "Linha {0}: {1}",
            ],
            Msg::StepFrom => [
                "From X {0} Y {1} Z {2}",
                "Desde X {0} Y {1} Z {2}",
                "Von X {0} Y {1} Z {2}",
                "De X {0} Y {1} Z {2}",
                "Da X {0} Y {1} Z {2}",
                "開始 X {0} Y {1} Z {2}",
                "从 X {0} Y {1} Z {2}",
                "De X {0} Y {1} Z {2}",
            ],
            Msg::StepTo => [
                "To X {0} Y {1} Z {2}",
                "Hasta X {0} Y {1} Z {2}",
                "Nach X {0} Y {1} Z {2}",
                "À X {0} Y {1} Z {2}",
                "A X {0} Y {1} Z {2}",
                "終了 X {0} Y {1} Z {2}",
                "到 X {0} Y {1} Z {2}",
                "Até X {0} Y {1} Z {2}",
            ],
            Msg::StepKindRapid => [
                "Rapid move, {0} mm in {1} s, starts at {2}",
                "Movimiento rápido, {0} mm en {1} s, empieza en {2}",
                "Eilgang, {0} mm in {1} s, beginnt bei {2}",
                "Mouvement rapide, {0} mm en {1} s, commence à {2}",
                "Movimento rapido, {0} mm in {1} s, inizia a {2}",
                "早送り、{0} mm、{1} 秒、開始 {2}",
                "快速移动，{0} mm，{1} 秒，开始于 {2}",
                "Movimento rápido, {0} mm em {1} s, começa a {2}",
            ],
            Msg::StepKindCut => [
                "Cutting move, {0} mm in {1} s, starts at {2}",
                "Movimiento de corte, {0} mm en {1} s, empieza en {2}",
                "Schnittbewegung, {0} mm in {1} s, beginnt bei {2}",
                "Mouvement de coupe, {0} mm en {1} s, commence à {2}",
                "Movimento di taglio, {0} mm in {1} s, inizia a {2}",
                "切削移動、{0} mm、{1} 秒、開始 {2}",
                "切削移动，{0} mm，{1} 秒，开始于 {2}",
                "Movimento de corte, {0} mm em {1} s, começa a {2}",
            ],
            Msg::StepFeedOn => [
                "Feed {0} mm/min, spindle on",
                "Avance {0} mm/min, husillo encendido",
                "Vorschub {0} mm/min, Spindel an",
                "Avance {0} mm/min, broche en marche",
                "Avanzamento {0} mm/min, mandrino acceso",
                "送り {0} mm/min、スピンドル ON",
                "进给 {0} mm/min，主轴开启",
                "Avanço {0} mm/min, fuso ligado",
            ],
            Msg::StepFeedOff => [
                "Feed {0} mm/min, spindle off",
                "Avance {0} mm/min, husillo apagado",
                "Vorschub {0} mm/min, Spindel aus",
                "Avance {0} mm/min, broche arrêtée",
                "Avanzamento {0} mm/min, mandrino spento",
                "送り {0} mm/min、スピンドル OFF",
                "进给 {0} mm/min，主轴关闭",
                "Avanço {0} mm/min, fuso desligado",
            ],
            Msg::ProgramOpened => [
                "{0}: {1} lines",
                "{0}: {1} líneas",
                "{0}: {1} Zeilen",
                "{0} : {1} lignes",
                "{0}: {1} righe",
                "{0}: {1} 行",
                "{0}：{1} 行",
                "{0}: {1} linhas",
            ],
            Msg::Analysed => [
                "{0} moves, {1} alerts, {2} warnings",
                "{0} movimientos, {1} alertas, {2} advertencias",
                "{0} Bewegungen, {1} Alarme, {2} Warnungen",
                "{0} mouvements, {1} alertes, {2} avertissements",
                "{0} movimenti, {1} allarmi, {2} avvisi",
                "{0} 件の移動、アラート {1} 件、警告 {2} 件",
                "{0} 次移动，{1} 个警报，{2} 个警告",
                "{0} movimentos, {1} alertas, {2} avisos",
            ],
            Msg::NeedProgram => [
                "Open a G-code program first",
                "Abre primero un programa G-code",
                "Öffnen Sie zuerst ein G-Code-Programm",
                "Ouvrez d'abord un programme G-code",
                "Apri prima un programma G-code",
                "先に G コード プログラムを開いてください",
                "请先打开 G 代码程序",
                "Abra primeiro um programa G-code",
            ],
            Msg::Truncated => [
                "The picture shows only the first {0} pieces",
                "El dibujo solo muestra las primeras {0} piezas",
                "Die Zeichnung zeigt nur die ersten {0} Stücke",
                "Le dessin ne montre que les {0} premiers morceaux",
                "Il disegno mostra solo i primi {0} pezzi",
                "図には最初の {0} 区間のみ表示します",
                "图中仅显示前 {0} 段",
                "O desenho mostra apenas as primeiras {0} peças",
            ],
        }
    }

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.words()[lang as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentence_has_a_text_in_each_of_the_eight_languages() {
        for m in Msg::ALL {
            for l in Lang::ALL {
                assert!(!m.tr(l).trim().is_empty(), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_list_of_sentences_has_no_repeats() {
        for (i, a) in Msg::ALL.iter().enumerate() {
            assert!(!Msg::ALL[i + 1..].contains(a), "{a:?} is listed twice");
        }
    }

    #[test]
    fn a_template_has_the_same_places_in_every_language() {
        for m in Msg::ALL {
            let places = |l: Lang| {
                (0..8)
                    .filter(|k| m.tr(l).contains(&format!("{{{k}}}")))
                    .count()
            };
            for l in Lang::ALL {
                assert_eq!(places(l), places(Lang::En), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_order_of_the_languages_matches_the_lists() {
        for (k, l) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(l as usize, k);
        }
        assert_eq!(Msg::Cancel.tr(Lang::Es), "Cancelar");
        assert_eq!(Msg::Cancel.tr(Lang::De), "Abbrechen");
        assert_eq!(Msg::Cancel.tr(Lang::Pt), "Cancelar");
    }

    #[test]
    fn a_language_is_found_from_the_code_of_the_system() {
        assert_eq!(Lang::from_code("es_ES.UTF-8"), Some(Lang::Es));
        assert_eq!(Lang::from_code("PT-pt"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_code("xx"), None);
        assert_eq!(Lang::from_code(""), None);
    }

    #[test]
    fn the_places_of_a_template_are_filled() {
        assert_eq!(fill("{1} and {0}", &["a", "b"]), "b and a");
        assert_eq!(fill("none", &["a"]), "none");
    }
}
