English: [README.md](README.md)

# Anzu

De Kindle a EPUB en una sola app para Windows, escrita en Rust.

Anzu encuentra los libros que descargaste con la app Kindle, comprueba que la
herramienta de desbloqueo es la última y admite tu versión de Kindle, la ejecuta y
convierte el resultado a EPUB. No hay que buscar descargas ni vigilar versiones.

El nombre viene de Anzu, el ave mesopotámica que robó la Tablilla de los Destinos.

## Requisitos

- Windows 10 u 11.
- La app **Amazon Kindle** de Microsoft Store, con tu sesión iniciada y los libros
  descargados.
- Conexión a internet para comprobar la herramienta antes de cada desbloqueo.

## Instalación

Descarga `anzu-<versión>-windows-x64-setup.exe` de la
[última publicación](https://github.com/ilien-dev/anzu/releases/latest) y ábrelo. Se instala
solo para tu usuario, en `%LOCALAPPDATA%\Programs\Anzu`, sin permisos de administrador. El
idioma que eliges en el instalador es el idioma con el que se abre Anzu. El instalador aún
no está firmado, así que Windows SmartScreen puede avisar la primera vez: elige «Más
información» y luego «Ejecutar de todas formas».

El instalador es el propio `anzu.exe`: con un nombre que contiene `setup` muestra la ventana
de instalación, se copia a la carpeta elegida, crea los accesos directos y registra Anzu en
las aplicaciones de Windows (`HKCU`, sin permisos de administrador). Para quitarlo,
desinstala Anzu desde Configuración › Aplicaciones; pregunta antes y pregunta aparte si
borrar su carpeta de datos.

Mientras está abierta, Anzu busca una versión nueva 30 segundos después de arrancar y luego
cada hora, con una sola petición pequeña a GitHub. Si la hay, ofrece descargarla, comprueba
su huella SHA-256 y, con «Reiniciar y actualizar», se cierra, deja que la versión nueva
ocupe su lugar (`--update`) y vuelve a abrirse. No se descarga ni se instala nada sin que
lo pidas.

## Uso

Abre Anzu. La ventana muestra:

- **Estado del sistema:** si está lista para desbloquear, con la versión de Kindle y
  la de la herramienta.
- **Tus libros**, en una tabla que se puede ordenar por cualquier columna. El estado
  de cada uno es un recorrido de tres tramos: con DRM, desbloqueado, EPUB listo.
- **Carpeta de destino** y un botón que dice lo que va a pasar, por ejemplo
  «Desbloquear y convertir 2 libros».

El título real de un libro se conoce cuando está desbloqueado; hasta entonces se
muestra su ASIN. Si la app Kindle está abierta, Anzu pide permiso antes de cerrarla.

El tema sigue al del sistema, o se fija en claro u oscuro. La interfaz está en inglés por
defecto y también en español. Ambos están en el panel de ajustes, tras el engranaje de la
cabecera, junto a la versión y «Buscar actualizaciones».

También convierte desde la terminal un libro ya desbloqueado:

```
anzu convert libro.kfx-zip -o C:\Libros
```

## Qué hace y qué no

| Parte | Quién la hace |
|---|---|
| Detectar Kindle y los libros | Anzu |
| Comprobar la última versión y la compatibilidad | Anzu |
| Descargar la herramienta y verificar su huella SHA-256 | Anzu |
| Quitar el DRM | La herramienta de [Satsuoni](https://github.com/Satsuoni/DeDRM_tools), como proceso externo |
| Leer el formato KFX y escribir el EPUB | Anzu, con código propio |

Anzu no contiene código de descifrado.

### Cuándo se bloquea el desbloqueo

Anzu se niega a desbloquear, y explica por qué, cuando:

- tu app Kindle es más nueva que la última herramienta publicada;
- no puede determinar con certeza si son compatibles;
- no hay conexión para comprobarlo;
- la descarga no trae huella SHA-256, o la huella no coincide.

La herramienta en caché se vuelve a comprobar en cada uso: si cambia la huella
publicada, o la de alguno de sus ejecutables, se descarga de nuevo.

Los libros ya desbloqueados se pueden convertir siempre.

### Alcance del conversor

Libros de texto: capítulos, encabezados, cursiva y negrita, enlaces internos y
externos, imágenes, portada, índice y metadatos. Las listas y las tablas conservan su
texto pero pierden la estructura. Quedan fuera los cómics, la maquetación fija y los
diccionarios. Lo que no se reconoce deja un aviso en el registro; no interrumpe la
conversión.

## Dónde guarda cosas

En `%LOCALAPPDATA%\anzu\`:

| Ruta | Contenido |
|---|---|
| `tool\<versión>\` | La herramienta descargada y su manifiesto de huellas |
| `unlocked\<ASIN>.kfx-zip` | Copias desbloqueadas de tus libros |
| `keys\anzu.k4i`, `work\` | **Secretos de tu cuenta de Amazon** |
| `settings.json` | Carpeta de destino, tema, idioma, títulos y libros convertidos |
| `updates\` | El instalador de una versión nueva, mientras espera al reinicio |

`keys\` y `work\` equivalen a una contraseña: no los compartas ni los subas a ningún
sitio. La herramienta de desbloqueo es un programa de terceros y puede dejar datos
propios fuera de esta carpeta; Anzu no lo controla.

La herramienta imprime secretos de la cuenta mientras trabaja. Anzu solo deja pasar al
registro las líneas de progreso que conoce, oculta en ellas cualquier cadena larga y
sustituye todo lo demás por un aviso.

Anzu nunca escribe en la carpeta de la app Kindle.

## Limitaciones conocidas de la versión 0.1.0

- El recorrido completo desde la ventana (desbloquear y convertir pulsando el botón)
  no se ha probado de principio a fin; sí cada parte por separado, con un libro real.
- Los EPUB no se han validado con `epubcheck`; se comprueba su integridad interna.
- No se puede cancelar una ejecución en curso. Si la herramienta deja de responder,
  Anzu la detiene pasados diez minutos sin actividad.
- La herramienta desbloquea todos los libros con DRM de la carpeta, no solo los
  marcados; los que ya estaban desbloqueados no se rehacen.
- La regla de compatibilidad se deduce del nombre del ejecutable de la herramienta.
  Si una publicación futura cambia ese nombre, Anzu bloqueará hasta que se actualice.
- El ejecutable aún no lleva icono propio en el Explorador; la ventana sí.

## Compilar

```
cargo build --release
cargo test --workspace
```

El ejecutable queda en `target\release\anzu.exe`.

## Estructura

| Crate | Responsabilidad |
|---|---|
| `anzu-kfx` | Lector de Ion binario y del contenedor KFX; modelo del libro; escritor de EPUB 3 |
| `anzu-core` | Detección de Kindle, publicaciones de GitHub, compatibilidad, descarga verificada, ejecución de la herramienta, biblioteca |
| `anzu` | Interfaz (`egui`) y línea de comandos |

`PRODUCT.md` y `.impeccable/` recogen el contexto del producto y la dirección de
diseño de la interfaz.

## Créditos

- **Satsuoni**, por la herramienta de desbloqueo.
- **John Howell** (jhowell), cuyo plugin KFX Input es la referencia del formato KFX.
- **JetBrains Mono**, la tipografía de la interfaz, bajo licencia SIL Open Font
  License 1.1 (`crates/anzu/assets/fonts/OFL.txt`).

## Licencia y aviso legal

GPL-3.0-or-later. Anzu no está aprobada por Amazon.

Quitar el DRM puede estar restringido donde vives. Úsala solo con libros que hayas
comprado y para leerlos en tus propios dispositivos.
