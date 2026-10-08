//! Langues de l'interface : français, anglais, espagnol, russe, arabe, chinois simplifié.
//! Chaque texte est une entrée de `Msg` avec ses six traductions, vérifiées à la compilation.
use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Fr,
    En,
    Es,
    Ru,
    Ar,
    Zh,
}

impl Lang {
    pub const ALL: [Lang; 6] = [Lang::Fr, Lang::En, Lang::Es, Lang::Ru, Lang::Ar, Lang::Zh];

    pub fn code(self) -> &'static str {
        ["fr", "en", "es", "ru", "ar", "zh"][self.index()]
    }

    /// Nom de la langue, écrit dans cette langue.
    pub fn name(self) -> &'static str {
        ["Français", "English", "Español", "Русский", "العربية", "中文"][self.index()]
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let c = code.trim().to_lowercase();
        Lang::ALL.into_iter().find(|l| c == l.code() || c.starts_with(&format!("{}-", l.code())) || c.starts_with(&format!("{}_", l.code())))
    }
}

static LANG: AtomicU8 = AtomicU8::new(1);

pub fn lang() -> Lang {
    Lang::ALL[(LANG.load(Ordering::Relaxed) as usize).min(5)]
}

pub fn set_lang(l: Lang) {
    LANG.store(l as u8, Ordering::Relaxed);
}

/// Langue de Windows (ou celle de la variable AUTOKEY_LANG pour les tests) ; anglais par défaut.
pub fn detect() -> Lang {
    if let Some(l) = std::env::var("AUTOKEY_LANG").ok().and_then(|v| Lang::from_code(&v)) {
        return l;
    }
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n > 0 {
        let name = String::from_utf16_lossy(&buf[..(n as usize).saturating_sub(1)]);
        if let Some(l) = Lang::from_code(&name) {
            return l;
        }
    }
    Lang::En
}

macro_rules! messages {
    ($($name:ident => [$fr:expr, $en:expr, $es:expr, $ru:expr, $ar:expr, $zh:expr]),* $(,)?) => {
        #[derive(Clone, Copy, Debug)]
        pub enum Msg { $($name),* }
        impl Msg {
            fn all(self) -> [&'static str; 6] {
                match self { $(Msg::$name => [$fr, $en, $es, $ru, $ar, $zh]),* }
            }
        }
    };
}

messages! {
    Subtitle => ["Une touche, un texte, à la milliseconde près.", "A key, a text, down to the millisecond.", "Una tecla, un texto, al milisegundo.", "Клавиша, текст — с точностью до миллисекунды.", "مفتاح أو نص، بدقة المللي ثانية.", "一个按键，一段文字，精确到毫秒。"],
    ListMode => ["Mode liste d'actions", "Action list mode", "Modo lista de acciones", "Режим списка действий", "وضع قائمة الإجراءات", "动作列表模式"],
    Donate => ["Soutenir {0}", "Support {0}", "Apoyar a {0}", "Поддержать {0}", "ادعم {0}", "支持 {0}"],
    Time => ["Heure", "Time", "Hora", "Время", "الوقت", "时间"],
    Date => ["Date", "Date", "Fecha", "Дата", "التاريخ", "日期"],
    Repeat => ["Répéter", "Repeat", "Repetir", "Повторять", "تكرار", "重复"],
    EveryX => ["× toutes les", "× every", "× cada", "× каждые", "× كل", "× 每"],
    Ms => ["ms", "ms", "ms", "мс", "مللي ث", "毫秒"],
    PickTarget => ["Choisir la zone de saisie", "Pick the input area", "Elegir la zona de entrada", "Выбрать поле ввода", "اختيار منطقة الإدخال", "选择输入区域"],
    Options => ["Options", "Options", "Opciones", "Параметры", "خيارات", "选项"],
    TargetInfo => ["Cible : {0} – « {1} »  (point {2}, {3})", "Target: {0} – “{1}”  (point {2}, {3})", "Destino: {0} – «{1}»  (punto {2}, {3})", "Цель: {0} – «{1}»  (точка {2}, {3})", "الهدف: {0} – {1} – النقطة {2}، {3}", "目标：{0} – “{1}”（位置 {2}, {3}）"],
    NoTarget => ["Aucune cible : la touche part dans la fenêtre active.", "No target: keys go to the active window.", "Sin destino: las teclas van a la ventana activa.", "Цель не задана: клавиши уйдут в активное окно.", "لا يوجد هدف: ستُرسل المفاتيح إلى النافذة النشطة.", "未设置目标：按键将发送到当前活动窗口。"],
    OptMinimize => ["Réduire la fenêtre au lancement", "Minimize the window when starting", "Minimizar la ventana al iniciar", "Свернуть окно при запуске", "تصغير النافذة عند البدء", "启动时最小化窗口"],
    OptGoTarget => ["Aller dans cette zone avant d'écrire", "Go to this area before typing", "Ir a esta zona antes de escribir", "Перейти в эту область перед вводом", "الانتقال إلى هذه المنطقة قبل الكتابة", "输入前先切换到该区域"],
    OptGoBack => ["Revenir ensuite où j'étais", "Then return to where I was", "Volver después adonde estaba", "Затем вернуться назад", "العودة بعد ذلك إلى حيث كنت", "完成后返回原来的位置"],
    TextHint => ["Tape ton texte ici… les touches cliquées s'ajoutent entre [crochets]", "Type your text here… clicked keys are added in [brackets]", "Escribe tu texto aquí… las teclas pulsadas se añaden entre [corchetes]", "Введите текст… нажатые клавиши добавляются в [квадратных скобках]", "اكتب نصك هنا… تُضاف المفاتيح التي تنقر عليها بين [أقواس]", "在此输入文字……点击的按键会以 [方括号] 形式加入"],
    HintKeyboard => ["Clique une touche du clavier pour l'insérer dans la ligne, ou tape du texte directement.", "Click a key on the keyboard to insert it in the line, or type text directly.", "Haz clic en una tecla del teclado para insertarla en la línea, o escribe el texto directamente.", "Нажмите клавишу на клавиатуре, чтобы вставить её в строку, или вводите текст напрямую.", "انقر على مفتاح في لوحة المفاتيح لإدراجه في السطر، أو اكتب النص مباشرة.", "点击键盘上的按键将其插入行中，或直接输入文字。"],
    HeldKeys => ["Touches maintenues : {0}", "Held keys: {0}", "Teclas mantenidas: {0}", "Удерживаемые клавиши: {0}", "المفاتيح المضغوطة: {0}", "按住的按键：{0}"],
    ListTitle => ["ACTIONS PLANIFIÉES", "SCHEDULED ACTIONS", "ACCIONES PROGRAMADAS", "ЗАПЛАНИРОВАННЫЕ ДЕЙСТВИЯ", "الإجراءات المجدولة", "计划动作"],
    ListHelp => ["Règle une action avec les cartes du dessus, puis ajoute-la. Double-clic sur une ligne pour la recharger.", "Set up an action with the cards above, then add it. Double-click a row to reload it.", "Configura una acción con las tarjetas de arriba y añádela. Doble clic en una fila para recargarla.", "Настройте действие в карточках выше и добавьте его. Двойной щелчок по строке — загрузить её обратно.", "اضبط إجراءً في البطاقات أعلاه ثم أضفه. انقر مرتين على صف لإعادة تحميله.", "在上方卡片中设置动作，然后添加。双击某一行可重新载入。"],
    ActiveWindow => ["fenêtre active", "active window", "ventana activa", "активное окно", "النافذة النشطة", "活动窗口"],
    HeldOnly => ["(touches maintenues)", "(held keys)", "(teclas mantenidas)", "(удерживаемые клавиши)", "(مفاتيح مضغوطة)", "（按住的按键）"],
    NextOccurrence => ["prochain", "next", "próximo", "ближайшее", "التالي", "下一次"],
    AddAction => ["Ajouter l'action ci-dessus", "Add the action above", "Añadir la acción de arriba", "Добавить действие выше", "إضافة الإجراء أعلاه", "添加上方的动作"],
    ReplaceRow => ["Remplacer la ligne sélectionnée", "Replace the selected row", "Reemplazar la fila seleccionada", "Заменить выбранную строку", "استبدال الصف المحدد", "替换所选行"],
    Delete => ["Supprimer", "Delete", "Eliminar", "Удалить", "حذف", "删除"],
    ClearAll => ["Tout vider", "Clear all", "Vaciar todo", "Очистить всё", "مسح الكل", "全部清空"],
    SelectRowFirst => ["Sélectionne d'abord une ligne de la liste.", "Select a row in the list first.", "Selecciona primero una fila de la lista.", "Сначала выберите строку в списке.", "حدّد صفًا في القائمة أولاً.", "请先在列表中选择一行。"],
    Arm => ["Armer", "Arm", "Armar", "Запустить", "تفعيل", "启动"],
    ArmList => ["Armer la liste", "Arm the list", "Armar la lista", "Запустить список", "تفعيل القائمة", "启动列表"],
    Cancel => ["Annuler", "Cancel", "Cancelar", "Отмена", "إلغاء", "取消"],
    Clear => ["Effacer", "Clear", "Borrar", "Очистить", "مسح", "清除"],
    TestBtn => ["Test (3 s)", "Test (3 s)", "Prueba (3 s)", "Тест (3 с)", "اختبار – 3 ث", "测试（3 秒）"],
    Keyboard => ["Clavier :", "Keyboard:", "Teclado:", "Клавиатура:", "لوحة المفاتيح:", "键盘："],
    EmergencyStop => ["Arrêt d'urgence : Ctrl + Alt + Échap", "Emergency stop: Ctrl + Alt + Esc", "Parada de emergencia: Ctrl + Alt + Esc", "Аварийная остановка: Ctrl + Alt + Esc", "إيقاف طارئ: Ctrl + Alt + Esc", "紧急停止：Ctrl + Alt + Esc"],
    NothingToSend => ["Rien à envoyer : tape du texte ou clique une touche.", "Nothing to send: type some text or click a key.", "Nada que enviar: escribe un texto o haz clic en una tecla.", "Нечего отправлять: введите текст или нажмите клавишу.", "لا شيء للإرسال: اكتب نصًا أو انقر على مفتاح.", "没有可发送的内容：请输入文字或点击按键。"],
    InvalidDateTime => ["Date ou heure invalide – {0}", "Invalid date or time – {0}", "Fecha u hora no válida – {0}", "Неверная дата или время – {0}", "تاريخ أو وقت غير صالح – {0}", "日期或时间无效 – {0}"],
    ListEmpty => ["La liste est vide : ajoute au moins une action.", "The list is empty: add at least one action.", "La lista está vacía: añade al menos una acción.", "Список пуст: добавьте хотя бы одно действие.", "القائمة فارغة: أضف إجراءً واحدًا على الأقل.", "列表为空：请至少添加一个动作。"],
    ActionInvalid => ["l'action {0} est invalide : {1}", "action {0} is invalid: {1}", "la acción {0} no es válida: {1}", "действие {0} недопустимо: {1}", "الإجراء {0} غير صالح: {1}", "动作 {0} 无效：{1}"],
    UnknownKey => ["Touche inconnue : [{0}]", "Unknown key: [{0}]", "Tecla desconocida: [{0}]", "Неизвестная клавиша: [{0}]", "مفتاح غير معروف: [{0}]", "未知按键：[{0}]"],
    DateFormatInvalid => ["date invalide (format JJ/MM/AAAA)", "invalid date (format DD/MM/YYYY)", "fecha no válida (formato DD/MM/AAAA)", "неверная дата (формат ДД/ММ/ГГГГ)", "تاريخ غير صالح (الصيغة DD/MM/YYYY)", "日期无效（格式 DD/MM/YYYY）"],
    TimeInvalid => ["heure invalide", "invalid time", "hora no válida", "неверное время", "وقت غير صالح", "时间无效"],
    LocalTimeNotFound => ["heure locale introuvable", "local time not found", "hora local no encontrada", "местное время не найдено", "تعذّر العثور على الوقت المحلي", "找不到本地时间"],
    DatePassed => ["Cette date/heure est déjà passée.", "This date/time has already passed.", "Esta fecha/hora ya ha pasado.", "Эта дата и время уже прошли.", "هذا التاريخ/الوقت قد مضى بالفعل.", "该日期/时间已经过去。"],
    WaitStatus => ["{0} – dans {1}", "{0} – in {1}", "{0} – en {1}", "{0} – через {1}", "{0} – بعد {1}", "{0} – 还有 {1}"],
    ActionName => ["Action {0}/{1} à {2}", "Action {0}/{1} at {2}", "Acción {0}/{1} a las {2}", "Действие {0}/{1} в {2}", "الإجراء {0}/{1} في {2}", "动作 {0}/{1}，时间 {2}"],
    TestName => ["Test", "Test", "Prueba", "Тест", "اختبار", "测试"],
    CountdownHMS => ["{0} h {1} min {2} s", "{0} h {1} min {2} s", "{0} h {1} min {2} s", "{0} ч {1} мин {2} с", "{0} س {1} د {2} ث", "{0} 小时 {1} 分 {2} 秒"],
    CountdownMS => ["{0} min {1} s", "{0} min {1} s", "{0} min {1} s", "{0} мин {1} с", "{0} د {1} ث", "{0} 分 {1} 秒"],
    CountdownS => ["{0} s", "{0} s", "{0} s", "{0} с", "{0} ث", "{0} 秒"],
    Cancelled => ["Annulé (arrêt d'urgence ou bouton).", "Cancelled (emergency stop or button).", "Cancelado (parada de emergencia o botón).", "Отменено (аварийная остановка или кнопка).", "تم الإلغاء (إيقاف طارئ أو زر).", "已取消（紧急停止或按钮）。"],
    Done => ["✔ Terminé à {0}", "✔ Done at {0}", "✔ Terminado a las {0}", "✔ Готово в {0}", "✔ اكتمل في {0}", "✔ 已于 {0} 完成"],
    DoneMany => ["✔ Terminé à {0} ({1} actions)", "✔ Done at {0} ({1} actions)", "✔ Terminado a las {0} ({1} acciones)", "✔ Готово в {0} ({1} действий)", "✔ اكتمل في {0} – {1} إجراءات", "✔ 已于 {0} 完成（{1} 个动作）"],
    PartialErr => ["⚠ {0}/{1} envoyée(s) – {2}", "⚠ {0}/{1} sent – {2}", "⚠ {0}/{1} enviada(s) – {2}", "⚠ отправлено {0}/{1} – {2}", "⚠ تم إرسال {0}/{1} – {2}", "⚠ 已发送 {0}/{1} – {2}"],
    ActionErr => ["action {0} : {1}", "action {0}: {1}", "acción {0}: {1}", "действие {0}: {1}", "الإجراء {0}: {1}", "动作 {0}：{1}"],
    Late => ["action {0} ignorée : en retard de {1} s", "action {0} skipped: {1} s late", "acción {0} omitida: {1} s de retraso", "действие {0} пропущено: опоздание {1} с", "تم تخطي الإجراء {0}: متأخر {1} ث", "动作 {0} 已跳过：迟了 {1} 秒"],
    KeysRefused => ["touches refusées par Windows (la fenêtre active est peut-être lancée en administrateur)", "keys refused by Windows (the active window may be running as administrator)", "teclas rechazadas por Windows (la ventana activa quizá se ejecuta como administrador)", "Windows отклонила нажатия (возможно, активное окно запущено от имени администратора)", "رفض Windows المفاتيح (ربما تعمل النافذة النشطة كمسؤول)", "Windows 拒绝了按键（活动窗口可能以管理员身份运行）"],
    ErrCursor => ["impossible de placer le curseur sur la zone cible", "cannot move the cursor to the target area", "no se puede colocar el cursor en la zona de destino", "не удалось переместить курсор в целевую область", "تعذّر وضع المؤشر على المنطقة المستهدفة", "无法将光标移到目标区域"],
    ErrClick => ["clic refusé par Windows (fenêtre cible lancée en administrateur ?)", "click refused by Windows (target window running as administrator?)", "clic rechazado por Windows (¿ventana de destino ejecutada como administrador?)", "Windows отклонила щелчок (окно-цель запущено от имени администратора?)", "رفض Windows النقرة (هل النافذة المستهدفة تعمل كمسؤول؟)", "Windows 拒绝了点击（目标窗口是否以管理员身份运行？）"],
    ErrWindowNotFound => ["fenêtre cible introuvable", "target window not found", "ventana de destino no encontrada", "окно-цель не найдено", "النافذة المستهدفة غير موجودة", "找不到目标窗口"],
    ErrCancelled => ["annulé", "cancelled", "cancelado", "отменено", "ملغى", "已取消"],
    ErrForeground => ["impossible de mettre la fenêtre au premier plan", "cannot bring the window to the front", "no se puede traer la ventana al primer plano", "не удалось вывести окно на передний план", "تعذّر إحضار النافذة إلى الأمام", "无法将窗口置于前台"],
    ErrCovered => ["la zone cible est masquée par une autre fenêtre", "the target area is covered by another window", "la zona de destino está tapada por otra ventana", "целевая область перекрыта другим окном", "المنطقة المستهدفة مغطّاة بنافذة أخرى", "目标区域被其他窗口遮挡"],
    PickBanner => ["Clique dans la zone de saisie à cibler   (Échap = annuler)", "Click in the input area to target   (Esc = cancel)", "Haz clic en la zona de entrada   (Esc = cancelar)", "Щёлкните в нужном поле ввода   (Esc = отмена)", "انقر في منطقة الإدخال المطلوبة   (Esc = إلغاء)", "请在目标输入区域内点击   （Esc = 取消）"],
    KeyEsc => ["Échap", "Esc", "Esc", "Esc", "Esc", "Esc"],
    KeyCaps => ["Verr.Maj", "Caps Lock", "Bloq Mayús", "Caps Lock", "Caps Lock", "大写锁定"],
    KeyEnter => ["Entrée", "Enter", "Intro", "Enter", "Enter", "回车"],
    KeySpace => ["Espace", "Space", "Espacio", "Пробел", "مسافة", "空格"],
    KeyShift => ["Maj", "Shift", "Mayús", "Shift", "Shift", "Shift"],
}

/// Texte dans la langue courante.
pub fn t(m: Msg) -> &'static str {
    m.all()[lang().index()]
}

/// Texte avec arguments `{0}`, `{1}`… En arabe, chaque argument est isolé (LRI … PDI) pour que les nombres
/// et les noms latins ne soient pas inversés dans une phrase écrite de droite à gauche.
pub fn tf(m: Msg, args: &[&dyn Display]) -> String {
    let mut s = t(m).to_string();
    for (i, a) in args.iter().enumerate() {
        let v = a.to_string();
        let v = if lang() == Lang::Ar { format!("\u{2066}{v}\u{2069}") } else { v };
        s = s.replace(&format!("{{{i}}}"), &v);
    }
    s
}
