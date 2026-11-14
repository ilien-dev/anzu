# Ventana principal de Anzu

Modo: Operate. El usuario viene a convertir sus libros; la pantalla es una herramienta.

## Direction contract

THESIS: Una consola de trabajo sobria: la tabla de libros es la pantalla. Rechaza la
rejilla de tarjetas iguales y el panel de tres métricas grandes que este tipo de app
suele llevar.

OWN-WORLD: «Consola ámbar», elegida por el usuario. Aporta cuatro cosas y solo esas:
tipografía monoespaciada (JetBrains Mono), densidad alta, paleta de tinta con ámbar
como único acento, y un rasgo propio. Esquinas casi rectas, líneas finas, sin sombras.
Tema claro «Papel frío»: fondo blanco frío #F3F5F7, paneles blancos, tinta #14171A,
ámbar tostado #B25E00; el ámbar nunca es fondo. Tema oscuro: carbón cálido #0E0F0C con
ámbar #FFB000. La cabecera es una banda de tinta oscura en ambos temas.

STORY: El usuario ve si el sistema está listo, reconoce sus libros por el título,
marca los que quiere y pulsa un botón que dice lo que va a pasar.

FIRST VIEWPORT: Banda de cabecera con logotipo, tema y «Volver a comprobar». Debajo,
una línea de estado. Después la tabla, que ocupa el resto: selección, título, ASIN,
tamaño y estado, con cabeceras ordenables. Al pie, carpeta de destino y la acción.
Todo en una columna centrada de 1160 px como máximo.

SIGNATURE: El estado de cada libro es un recorrido de tres tramos, con DRM →
desbloqueado → EPUB, que se va llenando.

REFUSALS: Nada de disfraz de terminal (corchetes, pseudoclaves, símbolo de sistema).
Controles estándar. Iconos dibujados, no glifos de texto.
