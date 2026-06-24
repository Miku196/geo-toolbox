"""
共享的 ReportLab PDF 报告工具函数。
供 flood_risk_pipeline.py 和 earthquake_pipeline.py 复用。
"""

from typing import Callable, Optional

from reportlab.lib.pagesizes import A4
from reportlab.lib.units import cm
from reportlab.lib.colors import HexColor, white, grey
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.enums import TA_CENTER, TA_LEFT, TA_JUSTIFY
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.cidfonts import UnicodeCIDFont


def register_chinese_font() -> str:
    """注册中文字体并返回字体名。回退到 Helvetica。"""
    try:
        pdfmetrics.registerFont(UnicodeCIDFont('STSong-Light'))
        return 'STSong-Light'
    except Exception:
        return 'Helvetica'


def build_pdf_styles(cn_font: str) -> dict:
    """构建标准的 PDF 段落样式字典。

    Returns:
        dict with keys: title, h1, h2, body, center
    """
    styles = getSampleStyleSheet()

    return {
        'title': ParagraphStyle(
            'CNTitle', parent=styles['Title'],
            fontName=cn_font, fontSize=22, leading=30,
            alignment=TA_CENTER, spaceAfter=20,
        ),
        'h1': ParagraphStyle(
            'CNH1', parent=styles['Heading1'],
            fontName=cn_font, fontSize=16, leading=22,
            spaceBefore=20, spaceAfter=10,
        ),
        'h2': ParagraphStyle(
            'CNH2', parent=styles['Heading2'],
            fontName=cn_font, fontSize=13, leading=18,
            spaceBefore=15, spaceAfter=8,
        ),
        'body': ParagraphStyle(
            'CNBody', parent=styles['Normal'],
            fontName=cn_font, fontSize=10, leading=16,
            alignment=TA_JUSTIFY,
        ),
        'center': ParagraphStyle(
            'CNCenter', parent=styles['Normal'],
            fontName=cn_font, fontSize=10, leading=16,
            alignment=TA_CENTER,
        ),
    }


def make_pdf_cover(
    story: list,
    title_lines: list[str],
    subtitle: str,
    date_text: str,
    source_text: str,
    tool_text: str,
    styles: dict,
) -> None:
    """添加 PDF 封面页到 story 列表。"""
    story.append(Spacer(1, 3 * cm))
    for line in title_lines:
        story.append(Paragraph(line, styles['title']))
    story.append(Spacer(1, 1 * cm))
    story.append(Paragraph(
        subtitle,
        ParagraphStyle('ENSub', parent=styles['body'], alignment=TA_CENTER, fontSize=12),
    ))
    story.append(Spacer(1, 2 * cm))
    story.append(Paragraph(
        f"评估日期: {date_text}",
        ParagraphStyle('Date', parent=styles['center'], fontSize=11),
    ))
    story.append(Paragraph(
        source_text,
        ParagraphStyle('Source', parent=styles['center'], fontSize=10, textColor=grey),
    ))
    story.append(Paragraph(
        tool_text,
        ParagraphStyle('Tools', parent=styles['center'], fontSize=10, textColor=grey),
    ))


def make_pdf_toc(story: list, items: list[str], styles: dict) -> None:
    """添加 PDF 目录页到 story 列表。"""
    story.append(Paragraph("目录", styles['h1']))
    for item in items:
        story.append(Paragraph(item, styles['body']))


def create_pdf_doc(output_path: str) -> SimpleDocTemplate:
    """创建标准的 A4 PDF 文档模板。"""
    return SimpleDocTemplate(
        output_path,
        pagesize=A4,
        rightMargin=2 * cm,
        leftMargin=2 * cm,
        topMargin=2 * cm,
        bottomMargin=2 * cm,
    )
