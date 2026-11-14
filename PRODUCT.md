# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Personas que compran libros en Kindle y quieren leerlos como EPUB en otros lectores.
La app está pensada para publicarse: quien la abre puede no ser técnico y no conoce
las herramientas que hay debajo. La mayoría tendrá de 1 a 20 libros descargados a la
vez; algunos tendrán cientos, y la lista debe seguir siendo manejable para ellos.

## Product Purpose

Anzu convierte a EPUB los libros descargados con la app Kindle de Microsoft Store,
sin que el usuario tenga que buscar herramientas, comprobar versiones ni tocar
carpetas. Éxito: abrir la app, ver los libros con su título, elegir cuáles y obtener
los EPUB en la carpeta elegida.

## Positioning

Anzu se niega a actuar cuando no puede garantizar que es seguro: comprueba antes de
cada desbloqueo que la herramienta es la última publicada, que su huella coincide y
que admite la versión de Kindle instalada. Si algo no cuadra, lo explica y no hace
nada. El conversor de KFX a EPUB es código propio.

## Operating Context

- Windows 10 u 11, con la app Amazon Kindle de Microsoft Store instalada y con sesión
  iniciada.
- El desbloqueo lo hace la herramienta de Satsuoni, que Anzu descarga y ejecuta como
  proceso externo. Anzu no contiene código de descifrado.
- La app Kindle debe estar cerrada durante el desbloqueo; Anzu pide permiso antes de
  cerrarla.
- Hace falta conexión para comprobar la herramienta. Convertir libros ya
  desbloqueados no la necesita.

## Capabilities and Constraints

- Aplicación de escritorio nativa para Windows, escrita en Rust con `egui`. No hay
  HTML ni navegador: el valor `web` de Platform solo indica que no sigue las guías de
  iOS ni de Android.
- Estados de un libro: con DRM, desbloqueado, convertido.
- El título real solo se conoce cuando el libro está desbloqueado; hasta entonces se
  muestra su ASIN.
- Convierte libros de texto. Cómics, maquetación fija y diccionarios quedan fuera.
- Interfaz en español.
- Tema claro y oscuro: sigue al sistema o lo fija el usuario.
- El contenido tiene un ancho máximo y queda centrado en monitores grandes.
- Sin decidir: búsqueda y ordenación en la lista para bibliotecas de cientos de
  libros; cancelar una ejecución en curso.

## Brand Commitments

- Nombre: Anzu, por el ave mesopotámica que robó la Tablilla de los Destinos.
- Logotipo: monograma A formado por dos alas abiertas sobre una tablilla.
- Estilo elegido por el usuario: «Consola ámbar» (monoespaciada, técnica, densa,
  acento ámbar). El tema claro no debe ser el típico fondo amarillento.

## Evidence on Hand

- Un libro real convertido de extremo a extremo (77 capítulos, verificación interna
  del EPUB sin problemas).
- Propuestas de interfaz y de logotipo aprobadas en canvas.
- No hay testimonios, cifras de uso ni capturas de terceros; no inventar ninguno.

## Product Principles

1. Ante la duda, no actuar y explicar por qué.
2. Nunca escribir en la carpeta de Kindle ni cerrar la app sin permiso.
3. Los secretos de la cuenta no se muestran ni se guardan en registros.
4. Un libro que falla no detiene a los demás.
5. Decir siempre qué va a pasar antes de que pase.
