/// TeXifier-style symbol palette categories. `rawValue` doubles as the
/// stable identifier; `titleKey` resolves the display name through
/// `Localizable.strings`. Mirrored by `symbols::SymbolCategory` in the Rust
/// `language-core` port — keep both lists identical for parity.
public enum SymbolCategory: String, CaseIterable, Hashable, Codable, Sendable {
    case greekLetters
    case binaryRelations
    case binaryOperators
    case bigOperators
    case arrows
    case delimiters
    case largeDelimiters
    case miscMaths
    case textModeSymbols
    case amsDelimiters
    case amsGreekHebrew
    case amsBinaryOperators
    case amsBinaryRelations
    case amsArrows

    /// `Localizable.strings` key for the category's picker label.
    public var titleKey: String {
        switch self {
        case .greekLetters: return "editor.symbol_cat_greek"
        case .binaryRelations: return "editor.symbol_cat_binary_relations"
        case .binaryOperators: return "editor.symbol_cat_binary_operators"
        case .bigOperators: return "editor.symbol_cat_big_operators"
        case .arrows: return "editor.symbol_cat_arrows"
        case .delimiters: return "editor.symbol_cat_delimiters"
        case .largeDelimiters: return "editor.symbol_cat_large_delimiters"
        case .miscMaths: return "editor.symbol_cat_misc_math"
        case .textModeSymbols: return "editor.symbol_cat_text_mode"
        case .amsDelimiters: return "editor.symbol_cat_ams_delimiters"
        case .amsGreekHebrew: return "editor.symbol_cat_ams_greek_hebrew"
        case .amsBinaryOperators: return "editor.symbol_cat_ams_binary_operators"
        case .amsBinaryRelations: return "editor.symbol_cat_ams_binary_relations"
        case .amsArrows: return "editor.symbol_cat_ams_arrows"
        }
    }
}

/// One palette cell: the rendered `glyph` shown large, the LaTeX `command`
/// inserted at the caret on click.
public struct TexSymbol: Hashable, Codable, Sendable {
    public let glyph: String
    public let command: String
    public let category: SymbolCategory

    public init(glyph: String, command: String, category: SymbolCategory) {
        self.glyph = glyph
        self.command = command
        self.category = category
    }
}

/// The static symbol catalogue — standard LaTeX plus the AMS groups, in a
/// fixed order so the palette renders identically on both platforms.
public enum TexSymbolCatalogue {
    public static let symbols: [TexSymbol] = [
        // ── Greek letters ──
        s("α", "\\alpha", .greekLetters), s("β", "\\beta", .greekLetters),
        s("γ", "\\gamma", .greekLetters), s("δ", "\\delta", .greekLetters),
        s("ϵ", "\\epsilon", .greekLetters), s("ε", "\\varepsilon", .greekLetters),
        s("ζ", "\\zeta", .greekLetters), s("η", "\\eta", .greekLetters),
        s("θ", "\\theta", .greekLetters), s("ϑ", "\\vartheta", .greekLetters),
        s("ι", "\\iota", .greekLetters), s("κ", "\\kappa", .greekLetters),
        s("λ", "\\lambda", .greekLetters), s("μ", "\\mu", .greekLetters),
        s("ν", "\\nu", .greekLetters), s("ξ", "\\xi", .greekLetters),
        s("π", "\\pi", .greekLetters), s("ϖ", "\\varpi", .greekLetters),
        s("ρ", "\\rho", .greekLetters), s("ϱ", "\\varrho", .greekLetters),
        s("σ", "\\sigma", .greekLetters), s("ς", "\\varsigma", .greekLetters),
        s("τ", "\\tau", .greekLetters), s("υ", "\\upsilon", .greekLetters),
        s("φ", "\\phi", .greekLetters), s("ϕ", "\\varphi", .greekLetters),
        s("χ", "\\chi", .greekLetters), s("ψ", "\\psi", .greekLetters),
        s("ω", "\\omega", .greekLetters),
        s("Γ", "\\Gamma", .greekLetters), s("Δ", "\\Delta", .greekLetters),
        s("Θ", "\\Theta", .greekLetters), s("Λ", "\\Lambda", .greekLetters),
        s("Ξ", "\\Xi", .greekLetters), s("Π", "\\Pi", .greekLetters),
        s("Σ", "\\Sigma", .greekLetters), s("Υ", "\\Upsilon", .greekLetters),
        s("Φ", "\\Phi", .greekLetters), s("Ψ", "\\Psi", .greekLetters),
        s("Ω", "\\Omega", .greekLetters),

        // ── Binary relations ──
        s("≤", "\\leq", .binaryRelations), s("≥", "\\geq", .binaryRelations),
        s("≠", "\\neq", .binaryRelations), s("≈", "\\approx", .binaryRelations),
        s("≡", "\\equiv", .binaryRelations), s("≅", "\\cong", .binaryRelations),
        s("≃", "\\simeq", .binaryRelations), s("∼", "\\sim", .binaryRelations),
        s("∝", "\\propto", .binaryRelations), s("≐", "\\doteq", .binaryRelations),
        s("≍", "\\asymp", .binaryRelations), s("≺", "\\prec", .binaryRelations),
        s("≻", "\\succ", .binaryRelations), s("⪯", "\\preceq", .binaryRelations),
        s("⪰", "\\succeq", .binaryRelations), s("≪", "\\ll", .binaryRelations),
        s("≫", "\\gg", .binaryRelations), s("⊂", "\\subset", .binaryRelations),
        s("⊃", "\\supset", .binaryRelations), s("⊆", "\\subseteq", .binaryRelations),
        s("⊇", "\\supseteq", .binaryRelations), s("⊏", "\\sqsubset", .binaryRelations),
        s("⊐", "\\sqsupset", .binaryRelations), s("⊑", "\\sqsubseteq", .binaryRelations),
        s("⊒", "\\sqsupseteq", .binaryRelations), s("∈", "\\in", .binaryRelations),
        s("∋", "\\ni", .binaryRelations), s("∉", "\\notin", .binaryRelations),
        s("⊢", "\\vdash", .binaryRelations), s("⊣", "\\dashv", .binaryRelations),
        s("⊨", "\\models", .binaryRelations), s("⊥", "\\perp", .binaryRelations),
        s("∣", "\\mid", .binaryRelations), s("∥", "\\parallel", .binaryRelations),
        s("⌣", "\\smile", .binaryRelations), s("⌢", "\\frown", .binaryRelations),
        s("⋈", "\\bowtie", .binaryRelations),

        // ── Binary operators ──
        s("±", "\\pm", .binaryOperators), s("∓", "\\mp", .binaryOperators),
        s("×", "\\times", .binaryOperators), s("÷", "\\div", .binaryOperators),
        s("·", "\\cdot", .binaryOperators), s("∪", "\\cup", .binaryOperators),
        s("∩", "\\cap", .binaryOperators), s("⊔", "\\sqcup", .binaryOperators),
        s("⊓", "\\sqcap", .binaryOperators), s("∨", "\\vee", .binaryOperators),
        s("∧", "\\wedge", .binaryOperators), s("⊕", "\\oplus", .binaryOperators),
        s("⊖", "\\ominus", .binaryOperators), s("⊗", "\\otimes", .binaryOperators),
        s("⊘", "\\oslash", .binaryOperators), s("⊙", "\\odot", .binaryOperators),
        s("∘", "\\circ", .binaryOperators), s("∗", "\\ast", .binaryOperators),
        s("⋆", "\\star", .binaryOperators), s("†", "\\dagger", .binaryOperators),
        s("‡", "\\ddagger", .binaryOperators), s("∖", "\\setminus", .binaryOperators),
        s("◁", "\\triangleleft", .binaryOperators), s("▷", "\\triangleright", .binaryOperators),
        s("○", "\\bigcirc", .binaryOperators), s("∙", "\\bullet", .binaryOperators),
        s("⊎", "\\uplus", .binaryOperators), s("⨿", "\\amalg", .binaryOperators),
        s("⋄", "\\diamond", .binaryOperators), s("≀", "\\wr", .binaryOperators),

        // ── Big operators ──
        s("∑", "\\sum", .bigOperators), s("∏", "\\prod", .bigOperators),
        s("∐", "\\coprod", .bigOperators), s("∫", "\\int", .bigOperators),
        s("∮", "\\oint", .bigOperators), s("⋃", "\\bigcup", .bigOperators),
        s("⋂", "\\bigcap", .bigOperators), s("⨆", "\\bigsqcup", .bigOperators),
        s("⋁", "\\bigvee", .bigOperators), s("⋀", "\\bigwedge", .bigOperators),
        s("⨁", "\\bigoplus", .bigOperators), s("⨂", "\\bigotimes", .bigOperators),
        s("⨀", "\\bigodot", .bigOperators), s("⨄", "\\biguplus", .bigOperators),

        // ── Arrows ──
        s("←", "\\leftarrow", .arrows), s("→", "\\rightarrow", .arrows),
        s("↑", "\\uparrow", .arrows), s("↓", "\\downarrow", .arrows),
        s("↕", "\\updownarrow", .arrows), s("⇐", "\\Leftarrow", .arrows),
        s("⇒", "\\Rightarrow", .arrows), s("⇑", "\\Uparrow", .arrows),
        s("⇓", "\\Downarrow", .arrows), s("⇕", "\\Updownarrow", .arrows),
        s("↔", "\\leftrightarrow", .arrows), s("⇔", "\\Leftrightarrow", .arrows),
        s("↦", "\\mapsto", .arrows), s("⟵", "\\longleftarrow", .arrows),
        s("⟶", "\\longrightarrow", .arrows), s("⟸", "\\Longleftarrow", .arrows),
        s("⟹", "\\Longrightarrow", .arrows), s("⟷", "\\longleftrightarrow", .arrows),
        s("⟺", "\\Longleftrightarrow", .arrows), s("⟼", "\\longmapsto", .arrows),
        s("↩", "\\hookleftarrow", .arrows), s("↪", "\\hookrightarrow", .arrows),
        s("↼", "\\leftharpoonup", .arrows), s("↽", "\\leftharpoondown", .arrows),
        s("⇀", "\\rightharpoonup", .arrows), s("⇁", "\\rightharpoondown", .arrows),
        s("⇌", "\\rightleftharpoons", .arrows), s("↗", "\\nearrow", .arrows),
        s("↘", "\\searrow", .arrows), s("↖", "\\nwarrow", .arrows),
        s("↙", "\\swarrow", .arrows), s("⇝", "\\leadsto", .arrows),
        s("←", "\\gets", .arrows), s("→", "\\to", .arrows),

        // ── Delimiters ──
        s("(", "(", .delimiters), s(")", ")", .delimiters),
        s("[", "[", .delimiters), s("]", "]", .delimiters),
        s("{", "\\{", .delimiters), s("}", "\\}", .delimiters),
        s("⟨", "\\langle", .delimiters), s("⟩", "\\rangle", .delimiters),
        s("|", "|", .delimiters), s("‖", "\\|", .delimiters),
        s("⌈", "\\lceil", .delimiters), s("⌉", "\\rceil", .delimiters),
        s("⌊", "\\lfloor", .delimiters), s("⌋", "\\rfloor", .delimiters),
        s("/", "/", .delimiters), s("\\", "\\backslash", .delimiters),
        s("↑", "\\uparrow", .delimiters), s("↓", "\\downarrow", .delimiters),
        s("↕", "\\updownarrow", .delimiters), s("⇑", "\\Uparrow", .delimiters),
        s("⇓", "\\Downarrow", .delimiters), s("⇕", "\\Updownarrow", .delimiters),

        // ── Large delimiters ──
        s("()", "\\left( \\right)", .largeDelimiters),
        s("[]", "\\left[ \\right]", .largeDelimiters),
        s("{}", "\\left\\{ \\right\\}", .largeDelimiters),
        s("||", "\\left| \\right|", .largeDelimiters),
        s("‖‖", "\\left\\| \\right\\|", .largeDelimiters),
        s("⟨⟩", "\\left\\langle \\right\\rangle", .largeDelimiters),
        s("⌈⌉", "\\left\\lceil \\right\\rceil", .largeDelimiters),
        s("⌊⌋", "\\left\\lfloor \\right\\rfloor", .largeDelimiters),
        s("↑↓", "\\left\\uparrow \\right\\downarrow", .largeDelimiters),
        s("(", "\\bigl(", .largeDelimiters), s(")", "\\bigr)", .largeDelimiters),
        s("(", "\\Bigl(", .largeDelimiters), s(")", "\\Bigr)", .largeDelimiters),
        s("[", "\\bigl[", .largeDelimiters), s("]", "\\bigr]", .largeDelimiters),
        s("{", "\\bigl\\{", .largeDelimiters), s("}", "\\bigr\\}", .largeDelimiters),
        s("⟨", "\\bigl\\langle", .largeDelimiters), s("⟩", "\\bigr\\rangle", .largeDelimiters),
        s("|", "\\big|", .largeDelimiters), s("‖", "\\big\\|", .largeDelimiters),

        // ── Miscellaneous math ──
        s("∀", "\\forall", .miscMaths), s("∃", "\\exists", .miscMaths),
        s("∄", "\\nexists", .miscMaths), s("∅", "\\emptyset", .miscMaths),
        s("∞", "\\infty", .miscMaths), s("∂", "\\partial", .miscMaths),
        s("∇", "\\nabla", .miscMaths), s("ℵ", "\\aleph", .miscMaths),
        s("ℏ", "\\hbar", .miscMaths), s("ℓ", "\\ell", .miscMaths),
        s("℘", "\\wp", .miscMaths), s("ℜ", "\\Re", .miscMaths),
        s("ℑ", "\\Im", .miscMaths), s("′", "\\prime", .miscMaths),
        s("√", "\\surd", .miscMaths), s("⊤", "\\top", .miscMaths),
        s("⊥", "\\bot", .miscMaths), s("∠", "\\angle", .miscMaths),
        s("△", "\\triangle", .miscMaths), s("¬", "\\neg", .miscMaths),
        s("♭", "\\flat", .miscMaths), s("♮", "\\natural", .miscMaths),
        s("♯", "\\sharp", .miscMaths), s("♣", "\\clubsuit", .miscMaths),
        s("♢", "\\diamondsuit", .miscMaths), s("♡", "\\heartsuit", .miscMaths),
        s("♠", "\\spadesuit", .miscMaths), s("ı", "\\imath", .miscMaths),
        s("ȷ", "\\jmath", .miscMaths),

        // ── Text-mode symbols ──
        s("§", "\\S", .textModeSymbols), s("¶", "\\P", .textModeSymbols),
        s("†", "\\dag", .textModeSymbols), s("‡", "\\ddag", .textModeSymbols),
        s("©", "\\copyright", .textModeSymbols), s("£", "\\pounds", .textModeSymbols),
        s("¥", "\\textyen", .textModeSymbols), s("€", "\\texteuro", .textModeSymbols),
        s("™", "\\texttrademark", .textModeSymbols), s("®", "\\textregistered", .textModeSymbols),
        s("°", "\\textdegree", .textModeSymbols), s("…", "\\dots", .textModeSymbols),
        s("—", "\\textemdash", .textModeSymbols), s("–", "\\textendash", .textModeSymbols),
        s("‘", "\\textquoteleft", .textModeSymbols), s("’", "\\textquoteright", .textModeSymbols),
        s("“", "\\textquotedblleft", .textModeSymbols), s("”", "\\textquotedblright", .textModeSymbols),
        s("¡", "\\textexclamdown", .textModeSymbols), s("¿", "\\textquestiondown", .textModeSymbols),
        s("<", "\\textless", .textModeSymbols), s(">", "\\textgreater", .textModeSymbols),
        s("|", "\\textbar", .textModeSymbols), s("\\", "\\textbackslash", .textModeSymbols),
        s("~", "\\textasciitilde", .textModeSymbols), s("^", "\\textasciicircum", .textModeSymbols),
        s("_", "\\textunderscore", .textModeSymbols), s("#", "\\#", .textModeSymbols),
        s("$", "\\$", .textModeSymbols), s("%", "\\%", .textModeSymbols),
        s("&", "\\&", .textModeSymbols), s("{", "\\{", .textModeSymbols),
        s("}", "\\}", .textModeSymbols),

        // ── AMS delimiters ──
        s("⌜", "\\ulcorner", .amsDelimiters), s("⌝", "\\urcorner", .amsDelimiters),
        s("⌞", "\\llcorner", .amsDelimiters), s("⌟", "\\lrcorner", .amsDelimiters),
        s("|", "\\lvert", .amsDelimiters), s("|", "\\rvert", .amsDelimiters),
        s("‖", "\\lVert", .amsDelimiters), s("‖", "\\rVert", .amsDelimiters),

        // ── AMS Greek & Hebrew ──
        s("ϝ", "\\digamma", .amsGreekHebrew), s("ϰ", "\\varkappa", .amsGreekHebrew),
        s("ב", "\\beth", .amsGreekHebrew), s("ד", "\\daleth", .amsGreekHebrew),
        s("ג", "\\gimel", .amsGreekHebrew),

        // ── AMS binary operators ──
        s("∔", "\\dotplus", .amsBinaryOperators), s("∖", "\\smallsetminus", .amsBinaryOperators),
        s("⋒", "\\Cap", .amsBinaryOperators), s("⋓", "\\Cup", .amsBinaryOperators),
        s("⊼", "\\barwedge", .amsBinaryOperators), s("⊻", "\\veebar", .amsBinaryOperators),
        s("⩎", "\\doublebarwedge", .amsBinaryOperators), s("⊟", "\\boxminus", .amsBinaryOperators),
        s("⊠", "\\boxtimes", .amsBinaryOperators), s("⊡", "\\boxdot", .amsBinaryOperators),
        s("⊞", "\\boxplus", .amsBinaryOperators), s("⋇", "\\divideontimes", .amsBinaryOperators),
        s("⋉", "\\ltimes", .amsBinaryOperators), s("⋊", "\\rtimes", .amsBinaryOperators),
        s("⋋", "\\leftthreetimes", .amsBinaryOperators), s("⋌", "\\rightthreetimes", .amsBinaryOperators),
        s("⋏", "\\curlywedge", .amsBinaryOperators), s("⋎", "\\curlyvee", .amsBinaryOperators),
        s("⊝", "\\circleddash", .amsBinaryOperators), s("⊛", "\\circledast", .amsBinaryOperators),
        s("⊚", "\\circledcirc", .amsBinaryOperators), s("⋅", "\\centerdot", .amsBinaryOperators),
        s("⊺", "\\intercal", .amsBinaryOperators),

        // ── AMS binary relations ──
        s("⩽", "\\leqslant", .amsBinaryRelations), s("⩾", "\\geqslant", .amsBinaryRelations),
        s("≲", "\\lesssim", .amsBinaryRelations), s("≳", "\\gtrsim", .amsBinaryRelations),
        s("⪅", "\\lessapprox", .amsBinaryRelations), s("⪆", "\\gtrapprox", .amsBinaryRelations),
        s("≊", "\\approxeq", .amsBinaryRelations), s("⋖", "\\lessdot", .amsBinaryRelations),
        s("⋗", "\\gtrdot", .amsBinaryRelations), s("⋘", "\\lll", .amsBinaryRelations),
        s("⋙", "\\ggg", .amsBinaryRelations), s("⋚", "\\lesseqgtr", .amsBinaryRelations),
        s("⋛", "\\gtreqless", .amsBinaryRelations), s("≑", "\\doteqdot", .amsBinaryRelations),
        s("≖", "\\eqcirc", .amsBinaryRelations), s("≗", "\\circeq", .amsBinaryRelations),
        s("≜", "\\triangleq", .amsBinaryRelations), s("≓", "\\risingdotseq", .amsBinaryRelations),
        s("≒", "\\fallingdotseq", .amsBinaryRelations), s("∽", "\\backsim", .amsBinaryRelations),
        s("≼", "\\preccurlyeq", .amsBinaryRelations), s("≽", "\\succcurlyeq", .amsBinaryRelations),
        s("⋞", "\\curlyeqprec", .amsBinaryRelations), s("⋟", "\\curlyeqsucc", .amsBinaryRelations),
        s("≾", "\\precsim", .amsBinaryRelations), s("≿", "\\succsim", .amsBinaryRelations),
        s("⫅", "\\subseteqq", .amsBinaryRelations), s("⫆", "\\supseteqq", .amsBinaryRelations),
        s("⋐", "\\Subset", .amsBinaryRelations), s("⋑", "\\Supset", .amsBinaryRelations),
        s("⊲", "\\vartriangleleft", .amsBinaryRelations), s("⊳", "\\vartriangleright", .amsBinaryRelations),
        s("⊴", "\\trianglelefteq", .amsBinaryRelations), s("⊵", "\\trianglerighteq", .amsBinaryRelations),
        s("⊨", "\\vDash", .amsBinaryRelations), s("⊩", "\\Vdash", .amsBinaryRelations),
        s("⊪", "\\Vvdash", .amsBinaryRelations), s("⌣", "\\smallsmile", .amsBinaryRelations),
        s("⌢", "\\smallfrown", .amsBinaryRelations), s("≏", "\\bumpeq", .amsBinaryRelations),
        s("≎", "\\Bumpeq", .amsBinaryRelations), s("≬", "\\between", .amsBinaryRelations),
        s("⋔", "\\pitchfork", .amsBinaryRelations), s("϶", "\\backepsilon", .amsBinaryRelations),
        s("∝", "\\varpropto", .amsBinaryRelations), s("◀", "\\blacktriangleleft", .amsBinaryRelations),
        s("▶", "\\blacktriangleright", .amsBinaryRelations), s("∴", "\\therefore", .amsBinaryRelations),
        s("∵", "\\because", .amsBinaryRelations),

        // ── AMS arrows ──
        s("⇠", "\\dashleftarrow", .amsArrows), s("⇢", "\\dashrightarrow", .amsArrows),
        s("⇇", "\\leftleftarrows", .amsArrows), s("⇉", "\\rightrightarrows", .amsArrows),
        s("⇆", "\\leftrightarrows", .amsArrows), s("⇄", "\\rightleftarrows", .amsArrows),
        s("⇚", "\\Lleftarrow", .amsArrows), s("⇛", "\\Rrightarrow", .amsArrows),
        s("↞", "\\twoheadleftarrow", .amsArrows), s("↠", "\\twoheadrightarrow", .amsArrows),
        s("↢", "\\leftarrowtail", .amsArrows), s("↣", "\\rightarrowtail", .amsArrows),
        s("↫", "\\looparrowleft", .amsArrows), s("↬", "\\looparrowright", .amsArrows),
        s("⇋", "\\leftrightharpoons", .amsArrows), s("⇌", "\\rightleftharpoons", .amsArrows),
        s("↶", "\\curvearrowleft", .amsArrows), s("↷", "\\curvearrowright", .amsArrows),
        s("↺", "\\circlearrowleft", .amsArrows), s("↻", "\\circlearrowright", .amsArrows),
        s("↰", "\\Lsh", .amsArrows), s("↱", "\\Rsh", .amsArrows),
        s("⇈", "\\upuparrows", .amsArrows), s("⇊", "\\downdownarrows", .amsArrows),
        s("↿", "\\upharpoonleft", .amsArrows), s("↾", "\\upharpoonright", .amsArrows),
        s("⇃", "\\downharpoonleft", .amsArrows), s("⇂", "\\downharpoonright", .amsArrows),
        s("⊸", "\\multimap", .amsArrows), s("⇝", "\\rightsquigarrow", .amsArrows),
        s("↭", "\\leftrightsquigarrow", .amsArrows), s("↚", "\\nleftarrow", .amsArrows),
        s("↛", "\\nrightarrow", .amsArrows), s("⇍", "\\nLeftarrow", .amsArrows),
        s("⇏", "\\nRightarrow", .amsArrows), s("↮", "\\nleftrightarrow", .amsArrows),
        s("⇎", "\\nLeftrightarrow", .amsArrows),
    ]

    /// Palette rows for `category`, preserving catalogue order.
    public static func symbols(in category: SymbolCategory) -> [TexSymbol] {
        symbols.filter { $0.category == category }
    }

    private static func s(_ glyph: String, _ command: String, _ category: SymbolCategory) -> TexSymbol {
        TexSymbol(glyph: glyph, command: command, category: category)
    }
}
