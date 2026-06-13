# Wie dieser Raytracer funktioniert

Dieses Dokument erklärt jeden Baustein **zweimal**:

> 🧒 **Intuition** — bildlich, ohne Formeln.
> 🎓 **Mathematik** — die exakte Rechnung dahinter.

Lies einfach die Ebene, die dich interessiert.

---

## Die Grundidee in einem Satz

**Wir tun so, als würden wir das Licht rückwärts verfolgen:** Für jeden Bildpunkt schießen wir einen Strahl von der Kamera in die Szene und fragen „auf welche Farbe blickt dieser Strahl?".

> 🧒 In echt fliegen unzählige Lichtstrahlen von Lampen los, prallen herum und einige landen zufällig in deinem Auge. Das alles nachzurechnen wäre Verschwendung — die meisten Strahlen treffen dein Auge nie. Trick: Wir starten **beim Auge** und gehen rückwärts. Genauso effizient, weil wir nur die Strahlen verfolgen, die wir wirklich sehen.

> 🎓 Das ist die physikalische Umkehrbarkeit von Lichtwegen (Helmholtz-Reziprozität). Statt das Lichttransport-Integral vorwärts von den Quellen zu lösen, sampeln wir es rückwärts von der Kamera.

---

## Das große Bild: die Render-Pipeline

```
für jeden Pixel (x, y):
    1. baue einen Strahl von der Kamera durch diesen Pixel
    2. finde das nächste Objekt, das der Strahl trifft        ← Geometrie
    3. bestimme die Farbe an diesem Punkt                     ← Licht & Material
    4. (bei Metall/Glas: schicke einen neuen Strahl, gehe zu 2) ← Rekursion
    5. schreibe die Farbe in den Bildpuffer
```

Der gesamte Code ist nur eine clevere Version der Funktion **`Pixel (x, y) → Farbe`**. Alles andere ist Detail.

---

# Teil 1 — Die Bausteine

## Vektoren (`vec3.rs`)

> 🧒 Ein Vektor ist ein Pfeil im Raum: drei Zahlen `(x, y, z)`. Wir benutzen denselben Pfeil für *drei* Dinge: einen **Ort** (wo ist etwas?), eine **Richtung** (wohin?) und sogar eine **Farbe** (`x, y, z` = Rot, Grün, Blau). Praktisch — ein Werkzeug für alles.

> 🎓 Ein Element des $\mathbb{R}^3$. Die zwei wichtigsten Operationen im ganzen Projekt:
>
> **Skalarprodukt** (misst, wie „gleichgerichtet" zwei Vektoren sind):
> $$\vec a \cdot \vec b = a_x b_x + a_y b_y + a_z b_z = \lVert \vec a\rVert\,\lVert \vec b\rVert \cos\theta$$
>
> **Kreuzprodukt** (liefert einen Vektor senkrecht zu beiden):
> $$\vec a \times \vec b = (a_y b_z - a_z b_y,\; a_z b_x - a_x b_z,\; a_x b_y - a_y b_x)$$
>
> **Länge** und **Normierung** (Pfeil auf Länge 1):
> $$\lVert \vec v\rVert = \sqrt{\vec v \cdot \vec v}, \qquad \hat v = \frac{\vec v}{\lVert \vec v\rVert}$$
>
> Das Skalarprodukt steckt später in Beleuchtung, Spiegelung und Brechung — fast die ganze Optik ist „verkleidetes Skalarprodukt".

## Strahlen (`ray.rs`)

> 🧒 Ein Strahl ist ein Startpunkt plus eine Richtung — wie ein Laserpointer. Gehst du ein Stückchen in die Richtung, kommst du an einen neuen Punkt.

> 🎓 Eine parametrisierte Halbgerade:
> $$P(t) = O + t\,\vec D, \qquad t \ge 0$$
> $O$ = Ursprung, $\vec D$ = Richtung. Größeres $t$ = weiter entfernt. Fast jede Frage im Raytracer lautet: „für welches $t$ passiert etwas?"

---

# Teil 2 — Was sieht ein Strahl?

## Die Kamera (`camera.rs`)

> 🧒 Stell dir ein Fenster vor, das einen Meter vor deinem Auge schwebt. Jeder Pixel ist ein Kästchen auf diesem Fenster. Um die Farbe eines Pixels zu finden, schaust du von deinem Auge **durch** sein Kästchen hinaus in die Welt. Drehst du den Kopf, dreht sich das Fenster mit.

> 🎓 Das **Lochkamera-Modell**. Eine Bildebene („Viewport") im Abstand der Brennweite $f$. Das Sichtfeld $\text{vfov}$ legt ihre Höhe fest:
> $$h_{\text{viewport}} = 2 f \tan\!\left(\frac{\text{vfov}}{2}\right), \qquad w_{\text{viewport}} = h_{\text{viewport}}\cdot \frac{\text{Bildbreite}}{\text{Bildhöhe}}$$
> Kleineres vfov → kleinerer Viewport → Zoom (Teleobjektiv). Für Pixelanteile $s,t\in[0,1]$ ist der Strahl:
> $$\vec D = \underbrace{P_{\text{oben-links}} + s\,\vec u + t\,\vec v}_{\text{Punkt auf dem Viewport}} - O$$
> mit den Kanten­vektoren $\vec u$ (rechts) und $\vec v$ (runter), die aus der Kamera-Orientierung kommen.

## Strahl trifft Kugel — das Herzstück (`hittable.rs`)

> 🧒 Eine Kugel ist „alle Punkte mit demselben Abstand $r$ vom Mittelpunkt". Wir fragen: Wandert unser Strahl irgendwann genau durch so einen Punkt? Setzt man die Strahl-Formel in die Kugel-Bedingung ein, fällt eine **quadratische Gleichung** heraus — dieselbe $ax^2+bx+c=0$ wie in der Schule. Ihre Lösungen sagen uns, *wo* der Strahl die Kugel durchsticht.

> 🎓 Kugel um $C$ mit Radius $r$: $\lVert P - C\rVert^2 = r^2$. Einsetzen von $P = O + t\vec D$ und $\vec{oc} = O - C$:
> $$(\vec D\cdot\vec D)\,t^2 + 2(\vec{oc}\cdot\vec D)\,t + (\vec{oc}\cdot\vec{oc} - r^2) = 0$$
> Mit $a = \vec D\cdot\vec D,\; b = 2\,\vec{oc}\cdot\vec D,\; c = \vec{oc}\cdot\vec{oc} - r^2$ entscheidet die **Diskriminante**:
> $$\Delta = b^2 - 4ac \quad\begin{cases} <0 & \text{Strahl verfehlt die Kugel} \\[2pt] \ge 0 & \text{zwei Schnittpunkte (Ein- und Austritt)} \end{cases}$$
> Den näheren nehmen wir: $\;t = \dfrac{-b - \sqrt{\Delta}}{2a}$. Andere Formen (Ebene, Dreieck) ersetzen nur diese eine Gleichung — der Rest des Renderers bleibt gleich.

## Die Normale

> 🧒 Die Normale ist der Pfeil, der an einem Oberflächenpunkt **senkrecht heraussteht** — die Richtung, in die die Fläche „schaut". Bei einer Kugel zeigt sie einfach vom Mittelpunkt zum Trefferpunkt. Sie ist der Schlüssel zur Beleuchtung: eine Fläche, die zum Licht schaut, ist hell.

> 🎓 $\vec n = \dfrac{P - C}{r}$ (durch $r$ teilen normiert ohne Wurzel). Wir speichern sie stets der Strahlrichtung **entgegengesetzt** und merken uns mit `front_face`, ob wir außen oder innen getroffen haben — das braucht das Glas.

---

# Teil 3 — Licht und Material

## Diffuses Licht: das Lambert-Gesetz (`Material::Lambertian`)

> 🧒 Halte ein Blatt Papier in die Sonne. Zeigt es direkt zur Sonne, ist es am hellsten. Kippst du es weg, wird es dunkler. Genau das rechnen wir aus: *Wie sehr schaut die Fläche zum Licht?*

> 🎓 Lamberts Kosinusgesetz: die Helligkeit ist proportional zu $\cos\theta$ zwischen Normale $\vec n$ und Lichtrichtung $\vec l$. Für Einheitsvektoren ist $\cos\theta$ **genau das Skalarprodukt**:
> $$\text{Helligkeit} = \max(0,\ \vec n \cdot \vec l)$$
> Das $\max(0,\cdot)$ verhindert „negatives Licht" für abgewandte Flächen. Unsere Farbe: $\;\text{albedo}\cdot(\text{ambient} + \vec n\cdot\vec l)$. (Der kleine Ambient-Term ersetzt das indirekte Streulicht, das wir aus Geschwindigkeitsgründen nicht voll simulieren.)

## Metall: Reflexion (`Material::Metal`)

> 🧒 Ein Spiegel wirft den Strahl im selben Winkel zurück, wie er ankam — wie ein Ball, der von einer Wand abprallt. Wir verfolgen diesen zurückgeworfenen Strahl einfach weiter und schauen, was *er* trifft. Deshalb sieht man im Metall die Umgebung.

> 🎓 Reflexion eines einfallenden Vektors $\vec d$ an der Normale $\vec n$:
> $$\vec r = \vec d - 2(\vec d \cdot \vec n)\,\vec n$$
> (Wir kehren den Anteil von $\vec d$ um, der in die Fläche zeigt.) Ein „Fuzz"-Term addiert optional einen kleinen Zufallsvektor → matter, gebürsteter Look.

## Glas: Brechung (`Material::Dielectric`)

> 🧒 Ein Strohhalm im Wasserglas sieht geknickt aus — Licht ändert beim Übergang in ein anderes Material die Richtung. Glas macht zweierlei gleichzeitig: ein bisschen spiegeln, der Rest geht hindurch und knickt ab. Wir verfolgen **beide** Strahlen und mischen sie.

> 🎓 **Snelliussches Brechungsgesetz:** $\;\eta_1 \sin\theta_1 = \eta_2 \sin\theta_2$. In Vektorform (mit $\eta = \eta_1/\eta_2$, $\cos\theta_1 = -\vec d\cdot\vec n$):
> $$\vec R_\perp = \eta\,(\vec d + \cos\theta_1\,\vec n), \qquad \vec R_\parallel = -\sqrt{1 - \lVert\vec R_\perp\rVert^2}\;\vec n, \qquad \vec R = \vec R_\perp + \vec R_\parallel$$
> **Totalreflexion:** ist $\eta\sin\theta_1 > 1$, gibt es keine Lösung — alles wird reflektiert.
> **Fresnel (Schlick-Näherung):** wie viel reflektiert wird, hängt vom Winkel ab (flacher Blick = mehr Spiegel, vgl. ein See am Horizont):
> $$R(\theta) = R_0 + (1-R_0)(1-\cos\theta)^5, \qquad R_0 = \left(\frac{\eta_1-\eta_2}{\eta_1+\eta_2}\right)^2$$
> Endfarbe = $R(\theta)\cdot(\text{Reflexion}) + (1-R(\theta))\cdot(\text{Brechung})$.

## Rekursion und die Tiefengrenze (`ray_color`)

> 🧒 Ein Strahl kann von Metall zu Glas zu Metall springen — wie ein Echo, das immer leiser wird. Irgendwann hören wir auf (nach max. 10 Sprüngen), sonst hallt es ewig.

> 🎓 `ray_color` ruft sich selbst auf. `MAX_DEPTH` begrenzt die Rekursion (sonst Endlosschleife zwischen zwei Spiegeln → Stack-Überlauf). Das ist eine **Whitted-artige** Auswertung: an jeder Fläche verfolgen wir gezielt die Spiegel-/Brechungsrichtung, statt das volle Lichtintegral zu sampeln (siehe ganz unten).

---

# Teil 4 — Schön und schnell

## Anti-Aliasing (`SAMPLES_PER_PIXEL`)

> 🧒 Ein Pixel ist ein kleines Quadrat, aber wir testen nur einen Punkt darin → harte, treppige Kanten. Lösung: mehrere Strahlen an leicht zufällige Stellen **im** Pixel schießen und die Farben mitteln. Der Rand wird zu einem weichen Mischton.

> 🎓 Monte-Carlo-Schätzung des Farb-Integrals über die Pixelfläche:
> $$C(x,y) \approx \frac{1}{N}\sum_{i=1}^{N} L\big(\text{Strahl durch } (x+\xi_i,\, y+\eta_i)\big), \quad \xi_i,\eta_i \sim \mathcal{U}[0,1)$$
> Der Fehler fällt mit $\mathcal{O}(1/\sqrt N)$.

## Gamma-Korrektur (`linear_to_gamma`)

> 🧒 Bildschirme „verstehen" Helligkeit anders als unsere Rechnung. Ohne Korrektur wirkt alles zu dunkel. Wir ziehen am Schluss die Wurzel aus jeder Farbe — fertig.

> 🎓 Anzeige $\approx \text{linear}^{1/\gamma}$ mit $\gamma\approx 2$, also $\sqrt{\cdot}$. Wichtig: **erst rechnen (linear), dann anzeigen (gamma)**. Mischen/Mitteln muss im Linearraum passieren, sonst stimmen die Helligkeiten nicht.

## Multithreading (`rayon`)

> 🧒 Jeder Pixel ist unabhängig — kein Pixel braucht das Ergebnis eines anderen. Also lassen wir alle CPU-Kerne gleichzeitig an verschiedenen Bildzeilen arbeiten. Bei 10 Kernen ≈ 10× schneller.

> 🎓 „Embarrassingly parallel": wir verteilen die Zeilen mit `par_chunks_mut`. Jede Zeile hat ihren **eigenen** Zufallsgenerator → kein geteilter Zustand, keine Sperren, fast lineare Skalierung.

## BVH — die Beschleunigungsstruktur (`aabb.rs`, `bvh.rs`)

> 🧒 Mit hunderten Kugeln wäre es dumm, jeden Strahl gegen *jede* Kugel zu testen. Stattdessen packen wir die Kugeln in Kisten, Kisten in größere Kisten — ein Baum. Trifft ein Strahl eine große Kiste nicht, können wir alles darin auf einen Schlag überspringen.

> 🎓 Eine **Bounding Volume Hierarchy** aus achsenparallelen Boxen (AABB). Der Box-Test ist die **Slab-Methode**: pro Achse ein Intervall $[t_{\text{lo}}, t_{\text{hi}}]$; der Strahl ist in der Box, wo sich alle drei Intervalle überlappen. Der Baum senkt die Kosten pro Strahl von
> $$\mathcal{O}(N) \quad\longrightarrow\quad \mathcal{O}(\log N)$$
> Bei ~470 Kugeln: von 470 Tests auf ~9. Messbar: 4K-Bild von **165 s → 16 s**.

---

# Die 10 Schritte, in denen wir es gebaut haben

| # | Schritt | Was dazu kam |
|---|---------|--------------|
| 1 | Fenster + Render-Schleife | ein Bildpuffer, pro Pixel eine Farbe |
| 2 | Strahlen + Kamera | Vektoren, ein Strahl pro Pixel, Himmelsverlauf |
| 3 | Erste Kugel | Strahl-Kugel-Schnitt (quadratische Gleichung), Normalen-Farbe |
| 4 | Beleuchtung | Lambert-Gesetz mit einer Lichtquelle |
| 5 | Welt aus Objekten | `Hittable`-Trait, Liste, nächster Treffer |
| 6 | Bewegliche Kamera | WASD + Pfeiltasten, Echtzeit-Navigation |
| 7 | Gamma + Anti-Aliasing | mehrere Strahlen pro Pixel, korrekte Helligkeit |
| 8 | Materialien | Metall (Reflexion) & Glas (Brechung), Rekursion |
| 9 | 4K + Multithreading | alle Kerne (`rayon`), 4K-PNG-Export, viele Kugeln |
| 10 | BVH | Box-Hierarchie → 10× schneller |

Das Schöne am Aufbau: **jeder Schritt baute auf dem vorigen auf, ohne ihn umzuwerfen.** Die Normale aus Schritt 3 fütterte Schritt 4. Das `Hittable`-Trait aus Schritt 5 nahm später die BVH klaglos auf. Die Render-Schleife aus Schritt 1 steht im Kern bis heute.

---

# Für den Mathe-Professor: der Bezug zur Rendering-Gleichung

Der „wahre" Lichttransport an einem Punkt $p$ in Richtung $\omega_o$ ist (Kajiya 1986):

$$L_o(p, \omega_o) = L_e(p, \omega_o) + \int_{\Omega} f_r(p, \omega_i, \omega_o)\, L_i(p, \omega_i)\, (\vec n \cdot \omega_i)\, \mathrm{d}\omega_i$$

ein rekursives Integral über die Hemisphäre $\Omega$, mit BRDF $f_r$ und dem Kosinus-Term $\vec n\cdot\omega_i$.

Dieser Raytracer ist eine bewusste **Whitted-Vereinfachung** davon, optimiert für **Echtzeit**:

- Das Hemisphären-Integral wird **nicht** voll gesampelt. Stattdessen werten wir pro Material nur ausgezeichnete Richtungen aus:
  - **diffus** → direkte Beleuchtung über $\max(0, \vec n\cdot\vec l)$ (der Kosinus-Term, ein Lichtterm statt Integral);
  - **spiegelnd** → die eine Reflexionsrichtung;
  - **dielektrisch** → Reflexion + Brechung, Fresnel-gewichtet.
- Der einzige Monte-Carlo-Teil ist das **Anti-Aliasing-Integral über die Pixelfläche** ($\mathcal{O}(1/\sqrt N)$).

**Bewusst weggelassen** (Preis der Echtzeit-Tauglichkeit): globale Beleuchtung / indirekte Diffus-Bounces, weiche Schatten, Kaustiken. Diese bräuchten echtes Path-Tracing — das volle Integral mit vielen Zufalls-Samples pro Pixel — und kostet Minuten statt Millisekunden pro Bild. Genau diese Erweiterung wäre der nächste Schritt von „Echtzeit-Raytracer" zu „Offline-Pfadverfolger".
